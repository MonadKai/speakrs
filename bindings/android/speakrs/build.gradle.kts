import java.util.Locale

plugins {
    id("com.android.library")
    `maven-publish`
}

group = "com.avencera"
version = "0.5.0"

val rustCrate = "speakrs-ffi"
val rustLibrary = "libspeakrs_ffi.so"
val onnxRuntimeAndroidVersion = "1.27.0"
val repoRoot = layout.projectDirectory.dir("../../..")
val ffiCrateDir = repoRoot.dir("crates/speakrs-ffi")
val generatedKotlinDir = layout.buildDirectory.dir("generated/source/uniffi/main/kotlin")
val generatedJniRoot = layout.buildDirectory.dir("generated/jniLibs")
val ndkVersionValue = "29.0.14206865"

data class AndroidRustTarget(
    val rustTarget: String,
    val abi: String,
    val clangPrefix: String,
)

val androidRustTargets = listOf(
    AndroidRustTarget("aarch64-linux-android", "arm64-v8a", "aarch64-linux-android24"),
    AndroidRustTarget("x86_64-linux-android", "x86_64", "x86_64-linux-android24"),
)

fun String.capitalized(): String =
    replaceFirstChar { if (it.isLowerCase()) it.titlecase(Locale.US) else it.toString() }

fun hostDynamicLibraryName(): String {
    val os = System.getProperty("os.name").lowercase(Locale.US)
    return when {
        os.contains("mac") -> "libspeakrs_ffi.dylib"
        os.contains("windows") -> "speakrs_ffi.dll"
        else -> "libspeakrs_ffi.so"
    }
}

fun androidSdkDir(): File =
    providers.environmentVariable("ANDROID_HOME")
        .orElse(providers.environmentVariable("ANDROID_SDK_ROOT"))
        .map(::File)
        .orNull
        ?: error("ANDROID_HOME or ANDROID_SDK_ROOT must point to the Android SDK")

fun androidNdkDir(): File =
    providers.environmentVariable("ANDROID_NDK_HOME")
        .map(::File)
        .orNull
        ?: androidSdkDir().resolve("ndk/$ndkVersionValue")

fun ndkHostTag(ndkDir: File): String =
    ndkDir.resolve("toolchains/llvm/prebuilt")
        .listFiles()
        ?.firstOrNull { it.isDirectory }
        ?.name
        ?: error("No LLVM toolchain found under ${ndkDir.resolve("toolchains/llvm/prebuilt")}")

fun linkerEnvName(target: String): String =
    "CARGO_TARGET_${target.uppercase(Locale.US).replace('-', '_')}_LINKER"

fun ccEnvName(target: String): String =
    "CC_${target.replace('-', '_')}"

fun arEnvName(target: String): String =
    "AR_${target.replace('-', '_')}"

fun toolchainBin(): File {
    val ndkDir = androidNdkDir()
    return ndkDir.resolve("toolchains/llvm/prebuilt/${ndkHostTag(ndkDir)}/bin")
}

fun configureAndroidCargoEnvironment(task: Exec) {
    val ndkDir = androidNdkDir()
    val toolchainBin = toolchainBin()
    task.environment("ANDROID_HOME", androidSdkDir().absolutePath)
    task.environment("ANDROID_NDK_HOME", ndkDir.absolutePath)
    for (target in androidRustTargets) {
        val clang = toolchainBin.resolve("${target.clangPrefix}-clang").absolutePath
        task.environment(
            linkerEnvName(target.rustTarget),
            clang,
        )
        task.environment(ccEnvName(target.rustTarget), clang)
        task.environment(arEnvName(target.rustTarget), toolchainBin.resolve("llvm-ar").absolutePath)
    }
}

val buildHostFfiForUniFfi by tasks.registering(Exec::class) {
    description = "Build the host speakrs UniFFI cdylib used to extract Kotlin metadata"
    workingDir(repoRoot.asFile)
    commandLine("cargo", "build", "-p", rustCrate, "--release")
    inputs.dir(ffiCrateDir)
    inputs.file(repoRoot.file("Cargo.lock"))
    outputs.file(repoRoot.file("target/release/${hostDynamicLibraryName()}"))
}

val generateUniFfiKotlin by tasks.registering(Exec::class) {
    description = "Generate Kotlin bindings from the speakrs UniFFI metadata"
    dependsOn(buildHostFfiForUniFfi)
    workingDir(repoRoot.asFile)

    val hostLibrary = repoRoot.file("target/release/${hostDynamicLibraryName()}")
    inputs.file(hostLibrary)
    inputs.file(ffiCrateDir.file("uniffi.toml"))
    outputs.dir(generatedKotlinDir)

    doFirst {
        generatedKotlinDir.get().asFile.deleteRecursively()
    }

    commandLine(
        "cargo",
        "run",
        "-p",
        rustCrate,
        "--features",
        "bindgen-cli",
        "--bin",
        "uniffi-bindgen",
        "--",
        "generate",
        "--language",
        "kotlin",
        "--metadata-no-deps",
        "--out-dir",
        generatedKotlinDir.get().asFile.absolutePath,
        "--library",
        hostLibrary.asFile.absolutePath,
    )
}

fun registerRustAndroidBuild(flavorName: String, cargoFeature: String): TaskProvider<Task> {
    val outputDir = generatedJniRoot.map { it.dir(flavorName) }
    val targetTasks = androidRustTargets.map { target ->
        tasks.register<Exec>("build${flavorName.capitalized()}${target.abi.replace("-", "").capitalized()}RustAndroid") {
            description = "Build speakrs native Android library for $flavorName ${target.abi}"
            workingDir(repoRoot.asFile)

            inputs.dir(ffiCrateDir)
            inputs.file(repoRoot.file("Cargo.lock"))
            outputs.file(outputDir.map { it.dir(target.abi).file(rustLibrary) })

            commandLine(
                "cargo",
                "build",
                "-p",
                rustCrate,
                "--release",
                "--no-default-features",
                "--features",
                "online,$cargoFeature",
                "--target",
                target.rustTarget,
            )
            configureAndroidCargoEnvironment(this)

            doLast {
                val source = repoRoot.file("target/${target.rustTarget}/release/$rustLibrary").asFile
                val destination = outputDir.get().dir(target.abi).file(rustLibrary).asFile
                if (!source.isFile) {
                    error("Expected native library was not built: $source")
                }
                destination.parentFile.mkdirs()
                source.copyTo(destination, overwrite = true)
            }
        }
    }

    return tasks.register("build${flavorName.capitalized()}RustAndroid") {
        description = "Build speakrs native Android libraries for the $flavorName flavor"
        dependsOn(targetTasks)
    }
}

val buildBundledOrtRustAndroid = registerRustAndroidBuild("bundledOrt", "bundled-ort")
val buildLeanRustAndroid = registerRustAndroidBuild("lean", "lean-ort")

android {
    namespace = "com.avencera.speakrs"
    compileSdk = 36
    ndkVersion = ndkVersionValue

    defaultConfig {
        minSdk = 24
        aarMetadata {
            minCompileSdk = 36
        }
    }

    flavorDimensions += "ort"
    productFlavors {
        create("bundledOrt") {
            dimension = "ort"
        }
        create("lean") {
            dimension = "ort"
        }
    }

    sourceSets {
        getByName("main") {
            kotlin.directories.add(generatedKotlinDir.get().asFile.absolutePath)
        }
        getByName("bundledOrt") {
            jniLibs.directories.add(generatedJniRoot.get().dir("bundledOrt").asFile.absolutePath)
        }
        getByName("lean") {
            jniLibs.directories.add(generatedJniRoot.get().dir("lean").asFile.absolutePath)
        }
    }

    publishing {
        singleVariant("bundledOrtRelease")
        singleVariant("leanRelease")
    }
}

tasks.matching { it.name == "preBundledOrtReleaseBuild" }.configureEach {
    dependsOn(generateUniFfiKotlin, buildBundledOrtRustAndroid)
}

tasks.matching { it.name == "preBundledOrtDebugBuild" }.configureEach {
    dependsOn(generateUniFfiKotlin, buildBundledOrtRustAndroid)
}

tasks.matching { it.name == "preLeanReleaseBuild" }.configureEach {
    dependsOn(generateUniFfiKotlin, buildLeanRustAndroid)
}

tasks.matching { it.name == "preLeanDebugBuild" }.configureEach {
    dependsOn(generateUniFfiKotlin, buildLeanRustAndroid)
}

tasks.matching { it.name.startsWith("compile") && it.name.endsWith("Kotlin") }.configureEach {
    dependsOn(generateUniFfiKotlin)
}

dependencies {
    api("androidx.annotation:annotation:1.9.1")
    api("net.java.dev.jna:jna:5.19.1@aar")
    api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
    add("bundledOrtApi", "com.microsoft.onnxruntime:onnxruntime-android:$onnxRuntimeAndroidVersion")
}

afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("bundledOrtRelease") {
                from(components["bundledOrtRelease"])
                artifactId = "speakrs-bundled-ort"
            }
            create<MavenPublication>("leanRelease") {
                from(components["leanRelease"])
                artifactId = "speakrs-lean"
            }
        }
    }
}
