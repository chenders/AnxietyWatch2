# Free Android traffic-inspection procedure

This is the no-paid-license traffic-inspection procedure for the app-comparison workflow.

## Recommended stack

- **mitmproxy/mitmweb:** open-source HTTP/HTTPS proxy for request/response metadata and payloads when the test app trusts the proxy CA.
- **Android Emulator and `adb`:** controlled execution environment.
- **`tcpdump`/TShark:** free packet-capture and metadata fallback when HTTPS cannot be intercepted.
- **Android Studio Network Inspector:** optional, primarily useful for an authorized debuggable build; it is not a substitute for inspecting a release APK that cannot be instrumented.

The current research workstation has `mitmproxy`/`mitmweb`, Android `adb`, the Android Emulator, and TShark installed. Verify versions before a future run and record them in the implementation-inspection report.

## Controlled sequence

Do not run this procedure against an app or artifact without authorization. Use a disposable emulator, synthetic/local data, and no production account.

1. Start mitmproxy on the host before installing or launching the target app. Use a loopback listener and port `8080`, for example:

   ```text
   mitmweb --listen-host 127.0.0.1 --listen-port 8080
   ```

   Keep the mitmweb UI bound to localhost unless a broader bind is specifically required and protected by the host firewall.

2. Start the Android Emulator and configure its global HTTP proxy before the target app starts:

   ```text
   adb shell settings put global http_proxy 10.0.2.2:8080
   adb shell settings get global http_proxy
   ```

   `10.0.2.2` is the Android Emulator's special route to the host loopback interface. Record the emulator name, Android version, proxy value, and mitmproxy version.

3. Install the target APK only after the proxy is configured. Do not create an account or log into a production service. If the authorized test requires a server, use a documented sandbox or synthetic account and record that boundary.

4. For HTTPS payload inspection, a disposable authorized emulator may use the mitmproxy CA through the emulator's normal certificate-install flow. Do not install the CA on a personal or production device. Do not use it to defeat certificate pinning. If the app rejects the CA or pins certificates, leave the traffic opaque and record the limitation.

5. Exercise only the approved workflows. Record requests, destinations, methods, data categories, response behavior, local storage, permissions, background work, and visible app behavior. Redact tokens, identifiers, personal data, and secrets before saving evidence.

6. When finished, clear the emulator proxy and verify it is no longer set:

   ```text
   adb shell settings delete global http_proxy
   adb shell settings get global http_proxy
   ```

   If the emulator requires the platform-specific `:0` reset instead, record the command used and verify the resulting empty/direct state.

## Packet-capture fallback

If HTTPS interception is unavailable or inappropriate, use an emulator packet capture rather than bypassing app protections. The Android Emulator supports a packet-capture output option at launch; inspect the resulting PCAP with TShark:

```text
emulator @<AVD_NAME> -tcpdump <capture-file>.pcap

tshark -r <capture-file>.pcap
```

Packet capture can establish timing, IP/DNS destinations, connection attempts, sizes, and transport behavior. It generally cannot reveal HTTPS request paths or payloads. Do not describe encrypted traffic as a confirmed API call merely from an IP address or hostname.

## Required evidence record

For each inspection, record:

- artifact name, package identifier, version/build, and SHA-256;
- emulator/Android version and `adb`/mitmproxy/TShark versions;
- proxy address and when it was applied;
- whether a mitmproxy CA was installed in the disposable emulator;
- whether certificate pinning or another limitation prevented decryption;
- test account/data boundary, if any;
- observed destinations and data categories, with secrets redacted;
- static findings versus runtime observations;
- cleanup/reset result; and
- unresolved questions and evidence gaps.

A failed interception is not evidence that the app is private, insecure, or free of network traffic. Label it **not observable under the authorized test configuration**.
