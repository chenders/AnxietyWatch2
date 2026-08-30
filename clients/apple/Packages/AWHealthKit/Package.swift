// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "AWHealthKit",
  platforms: [.macOS(.v14), .iOS(.v18)],
  products: [.library(name: "AWHealthKit", targets: ["AWHealthKit"])],
  dependencies: [
    .package(path: "../AWClientCore"),
    .package(path: "../AWTransport"),
  ],
  targets: [
    .target(name: "AWHealthKit", dependencies: ["AWClientCore", "AWTransport"]),
    .testTarget(name: "AWHealthKitTests", dependencies: ["AWHealthKit"]),
  ]
)
