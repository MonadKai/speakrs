// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "Speakrs",
    platforms: [
        .iOS(.v15),
    ],
    products: [
        .library(
            name: "Speakrs",
            targets: ["Speakrs"]
        ),
    ],
    targets: [
        .target(
            name: "Speakrs",
            dependencies: ["speakrs_ffiFFI"],
            path: "Sources/Speakrs"
        ),
        .binaryTarget(
            name: "speakrs_ffiFFI",
            path: "artifacts/speakrs_ffiFFI.xcframework"
        ),
    ]
)
