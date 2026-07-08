plugins {
    id("com.android.application")
}

val repoRoot = layout.projectDirectory.dir("../../..")
val generatedAssetsDir = layout.buildDirectory.dir("generated/assets/main")

val copySampleAudio by tasks.registering(Copy::class) {
    from(repoRoot.file("fixtures/test_short.wav"))
    into(generatedAssetsDir)
}

android {
    namespace = "com.avencera.speakrs.sample"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.avencera.speakrs.sample"
        minSdk = 24
        targetSdk = 36
        versionCode = 1
        versionName = "0.5.0"
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
            assets.directories.add(generatedAssetsDir.get().asFile.absolutePath)
        }
    }
}

tasks.matching { it.name == "preBuild" }.configureEach {
    dependsOn(copySampleAudio)
}

dependencies {
    implementation(project(":speakrs"))
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.11.0")
}
