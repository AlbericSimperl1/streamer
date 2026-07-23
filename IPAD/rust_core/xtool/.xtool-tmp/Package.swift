// swift-tools-version: 6.0
import PackageDescription
let package = Package(
    name: "streamer-Builder",
    platforms: [
        .iOS("26"),
    ],
    dependencies: [
        .package(name: "RootPackage", path: "../.."),
    ],
    targets: [
        .executableTarget(
    name: "streamer-App",
    dependencies: [
        .product(name: "streamer", package: "RootPackage"),
    ],
    linkerSettings: [
    .unsafeFlags([
        "-Xlinker", "-rpath", "-Xlinker", "@executable_path/Frameworks",
    ]),
]
)
    ]
)
