// swift-tools-version: 6.0
// swift-tools-version: 5.9

import Foundation
import PackageDescription

// Bepaal de absolute map waar dit Package.swift bestand in staat
let packageDir = URL(fileURLWithPath: #file).deletingLastPathComponent().path

let package = Package(
    name: "hyprClient",
    platforms: [
        .iOS("26")
    ],
    products: [
        .library(
            name: "hyprClient",
            targets: ["hyprClient"]
        )
    ],
    targets: [
        .target(
            name: "CRustCore",
            path: "Sources/CRustCore",
            publicHeadersPath: "include"
        ),
        .target(
            name: "hyprClient",
            dependencies: ["CRustCore"],
            path: "Sources/hyprClient",
            linkerSettings: [
                // Geef direct het absolute pad naar het bestand, niet de map!
                .unsafeFlags(["/home/alberic/streamer/IPAD/Sources/CRustCore/lib/librust_core.a"])
            ]
        ),
    ]
)
