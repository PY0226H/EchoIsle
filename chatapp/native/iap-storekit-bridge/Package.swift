// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "iap-storekit-bridge",
    platforms: [
        .macOS(.v13),
        .iOS(.v15),
    ],
    products: [
        .executable(
            name: "iap-storekit-bridge",
            targets: ["iap-storekit-bridge"]
        ),
    ],
    targets: [
        .executableTarget(
            name: "iap-storekit-bridge",
            path: "Sources"
        ),
    ]
)
