// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "AWClientCore",
  platforms: [.macOS(.v14), .iOS(.v18), .watchOS(.v11)],
  products: [.library(name: "AWClientCore", targets: ["AWClientCore"])],
  targets: [
    .target(name: "AWClientCore"),
    .testTarget(name: "AWClientCoreTests", dependencies: ["AWClientCore"]),
  ]
)
