#!/usr/bin/env python3
import sys
import time
import json
import os

LOG_FILE = os.path.expanduser("~/vitals/spike/night1.log")
STATUS_FILE = os.path.expanduser("~/anxietywatch-panel/docs/design/pi-panel/direction-a-clinical/status.json")

def main():
    os.makedirs(os.path.dirname(LOG_FILE), exist_ok=True)

    with open(LOG_FILE, "a") as log:
        for line in sys.stdin:
            # hr-spike format: <unix_epoch_seconds_float> <hr_bpm> <rr_ms,rr_ms,...>
            line = line.strip()
            if not line:
                continue

            # Write to the histogram log
            log.write(line + "\n")
            log.flush()

            # Parse for status.json
            parts = line.split()
            if len(parts) >= 2:
                try:
                    ts = float(parts[0])
                    hr = int(parts[1])

                    # Read current status.json if it exists to preserve EMAY data, or just create a new one
                    status = {}
                    if os.path.exists(STATUS_FILE):
                        try:
                            with open(STATUS_FILE, "r") as f:
                                status = json.load(f)
                        except:
                            pass

                    # Update status with Polar H10 data
                    status["polar_hr"] = hr
                    status["polar_at"] = ts
                    status["polar_link"] = "BLE"
                    status["written_at"] = time.time()

                    with open(STATUS_FILE, "w") as f:
                        json.dump(status, f)

                except Exception as e:
                    print(f"Error parsing line: {e}", file=sys.stderr)

if __name__ == "__main__":
    main()
