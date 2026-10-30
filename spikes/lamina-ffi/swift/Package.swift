// swift-tools-version:5.9
// Spike L6a: the Swift package shape ADR 0029 proposes: a binary target (the XCFramework with
// the static library and the C header), the generated bindings as an internal target, and a
// small hand-written `Lamina` target that is the public API. scripts/build-ios.sh fills build/.
import PackageDescription

let package = Package(
    name: "Lamina",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [.library(name: "Lamina", targets: ["Lamina"])],
    targets: [
        .binaryTarget(name: "LaminaFFIFFI", path: "build/LaminaFFIFFI.xcframework"),
        .target(name: "LaminaFFI", dependencies: ["LaminaFFIFFI"], path: "build/Sources/LaminaFFI"),
        .target(name: "Lamina", dependencies: ["LaminaFFI"], path: "Sources/Lamina"),
        .testTarget(name: "LaminaTests", dependencies: ["Lamina", "LaminaFFI"], path: "Tests/LaminaTests"),
    ]
)
