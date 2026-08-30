// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "AWDomain",
  platforms: [.macOS(.v14), .iOS(.v18), .watchOS(.v11)],
  products: [.library(name: "AWDomain", targets: ["AWDomain"])],
  dependencies: [.package(path: "../AWClientCore")],
  targets: [
    .target(name: "AWDomain", dependencies: ["AWClientCore"]),
    .testTarget(name: "AWDomainTests", dependencies: ["AWDomain"]),
  ]
)
