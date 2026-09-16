// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "AWTransport",
  platforms: [.macOS(.v14), .iOS(.v18), .watchOS(.v11)],
  products: [.library(name: "AWTransport", targets: ["AWTransport"])],
  dependencies: [
    .package(path: "../AWClientCore"),
    .package(path: "../AWOutbox"),
  ],
  targets: [
    .target(name: "AWTransport", dependencies: ["AWClientCore", "AWOutbox"]),
    .testTarget(name: "AWTransportTests", dependencies: ["AWTransport"]),
  ]
)
