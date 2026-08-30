// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "AWOutbox",
  platforms: [.macOS(.v14), .iOS(.v18)],
  products: [.library(name: "AWOutbox", targets: ["AWOutbox"])],
  dependencies: [.package(path: "../AWClientCore")],
  targets: [
    .target(name: "AWOutbox", dependencies: ["AWClientCore"]),
    .testTarget(name: "AWOutboxTests", dependencies: ["AWOutbox"]),
  ]
)
