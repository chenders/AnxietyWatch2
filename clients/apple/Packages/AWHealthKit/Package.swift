// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "AWHealthKit",
  platforms: [.macOS(.v14), .iOS(.v18)],
  products: [.library(name: "AWHealthKit", targets: ["AWHealthKit"])],
  targets: [
    .target(name: "AWHealthKit"),
    .testTarget(name: "AWHealthKitTests", dependencies: ["AWHealthKit"]),
  ]
)
