// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "AWDomain",
  platforms: [.macOS(.v14), .iOS(.v18), .watchOS(.v11)],
  products: [.library(name: "AWDomain", targets: ["AWDomain"])],
  targets: [
    .target(name: "AWDomain"),
    .testTarget(name: "AWDomainTests", dependencies: ["AWDomain"]),
  ]
)
