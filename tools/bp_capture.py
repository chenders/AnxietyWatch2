#!/usr/bin/env python3
"""Capture A&D UA-651BLE 0x2A35 indications via the BlueZ D-Bus API.

Why not bluetoothctl: its `connect` issued no LE Create Connection at all.
Confirmed against btmon -- the cuff produced 269 advertising reports while the
ATT trace stayed completely empty. bluetoothctl also returned empty scan output
to a pipe and dropped into the wrong submenu on `menu gatt`.

The cuff is a store-and-forward peripheral: its radio is dark except for a few
seconds after a measurement, when it advertises, connects to its bonded host,
waits for a CCCD write, indicates its stored readings, and disconnects. So the
kernel accept-list (btmgmt add-device -a 2) does the connecting -- a userspace
poll cannot reliably catch a 5-second window -- and this program only reacts.

0x2A35 is INDICATE, not NOTIFY. StartNotify() reads the characteristic's own
flags and writes the correct CCCD bit (0x0002). Writing 0x0001 by hand returns
success and then delivers nothing, forever.
"""
import json
import os
import re
import sys
import time
import traceback
import dbus
import dbus.mainloop.glib
from gi.repository import GLib

BLUEZ, OM = "org.bluez", "org.freedesktop.DBus.ObjectManager"
PROPS, CHRC, DEV = ("org.freedesktop.DBus.Properties",
                    "org.bluez.GattCharacteristic1", "org.bluez.Device1")

BPM = "00002a35-0000-1000-8000-00805f9b34fb"   # Blood Pressure Measurement
ICP = "00002a36-0000-1000-8000-00805f9b34fb"   # Intermediate Cuff Pressure
WANT = {BPM: "2A35 BloodPressureMeasurement", ICP: "2A36 IntermediateCuffPressure"}

# Plausibility bounds. NOT for clamping -- an out-of-range value is reported and
# kept, never silently corrected. They exist so a decode that produced 1.35e9
# mmHg (a nonzero SFLOAT exponent, which real hardware has never emitted here)
# cannot be transcribed into a fixture file as "hardware-provenanced".
PLAUSIBLE = {"systolic": (30.0, 300.0), "diastolic": (10.0, 200.0),
             "map": (20.0, 250.0), "pulse_bpm": (20.0, 250.0)}
KPA_TO_MMHG = 7.50062

CUFF = sys.argv[1] if len(sys.argv) > 1 else "00:09:1F:98:EB:CF"
if not re.fullmatch(r"(?:[0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}", CUFF):
    sys.exit(f"not a MAC address: {CUFF!r}")
# BlueZ object paths use UPPERCASE hex. Matching the argument verbatim meant a
# lowercase MAC made every path filter false forever -- silently, at exit 0,
# indistinguishable from a correct run that is waiting for a measurement. The
# lowercase form is exactly what the kernel prints, so it is the form an
# operator debugging this device is most likely to paste.
CUFF_PATH_FRAG = "dev_" + CUFF.upper().replace(":", "_")

# Frames are unrecoverable: the kernel ACKs the indication before this process
# sees it, and the cuff deletes its stored copy on that ACK. stdout is not
# storage -- a closed pipe or a full disk raises inside the handler, D-Bus
# swallows it, and the measurement is gone. Write durably BEFORE pretty-printing.
LOG_PATH = os.environ.get("BP_CAPTURE_LOG", "/var/lib/vitals/bp-capture.jsonl")
try:
    os.makedirs(os.path.dirname(LOG_PATH), exist_ok=True)
    LOG = open(LOG_PATH, "a", encoding="utf-8")
except OSError as exc:
    sys.exit(f"cannot open durable log {LOG_PATH}: {exc}. Refusing to run — "
             f"a capture with nowhere to land is worse than no capture.")

CAPTURED = 0


def persist(record):
    """Append one frame durably. fsync per frame: four readings a day."""
    LOG.write(json.dumps(record, sort_keys=True) + "\n")
    LOG.flush()
    os.fsync(LOG.fileno())

# IEEE-11073 16-bit SFLOAT. NOT an IEEE-754 half float -- a decoder that
# assumes 754 produces plausible-looking wrong numbers rather than failing.
SPECIAL = {0x07FF: "NaN", 0x0800: "NRes", 0x07FE: "+INF", 0x0802: "-INF", 0x0801: "Reserved"}

def sfloat(raw):
    mant, exp = raw & 0x0FFF, (raw >> 12) & 0x0F
    if mant in SPECIAL:
        return None, SPECIAL[mant]
    if mant >= 0x0800:
        mant -= 0x1000
    if exp >= 0x0008:
        exp -= 0x10
    return mant * (10.0 ** exp), None

def decode_2a35(b):
    """Returns (dict, error). Every length check is required: radio input.

    On a truncation error the partially-decoded fields are still returned, so
    the operator sees the vitals that did decode. The cuff has already deleted
    the record; printing only "truncated status" throws away three good values
    from the only readable view of a measurement that no longer exists anywhere.

    A sentinel is recorded in `_absent`, never coerced into the value field.
    Emitting the string "<NRes>" where a float belongs made a cuff explicitly
    reporting *no result* indistinguishable from a reading, in the very tool
    whose output becomes the fixture ground truth.
    """
    if len(b) < 7:
        return None, f"too short: {len(b)} bytes, need >= 7"
    out, flags, off = {}, b[0], 1
    absent = {}
    out["_flags"] = f"0x{flags:02X}"
    kpa = bool(flags & 0x01)
    # Convert at the decode boundary so nothing downstream ever sees kPa.
    out["units"] = "mmHg"
    if kpa:
        out["_wire_units"] = "kPa"

    def put(name, raw):
        val, special = sfloat(raw)
        if special is not None or val is None:
            absent[name] = special or "None"
            return
        if kpa and name != "pulse_bpm":
            val *= KPA_TO_MMHG
        out[name] = val

    for name in ("systolic", "diastolic", "map"):
        put(name, b[off] | (b[off + 1] << 8)); off += 2
    if flags & 0x02:                                    # timestamp, 7 bytes
        if len(b) < off + 7:
            return out, "truncated timestamp"
        yr = b[off] | (b[off + 1] << 8)
        out["timestamp"] = (f"{yr:04d}-{b[off+2]:02d}-{b[off+3]:02d} "
                            f"{b[off+4]:02d}:{b[off+5]:02d}:{b[off+6]:02d}")
        # An unset cuff clock reports zeros. That is not a date.
        if not (1900 <= yr <= 2200 and 1 <= b[off + 2] <= 12 and 1 <= b[off + 3] <= 31):
            out["_timestamp_implausible"] = True
        off += 7
    if flags & 0x04:                                    # pulse rate, SFLOAT
        if len(b) < off + 2:
            return out, "truncated pulse"
        put("pulse_bpm", b[off] | (b[off + 1] << 8)); off += 2
    if flags & 0x08:                                    # user id
        if len(b) < off + 1:
            return out, "truncated user id"
        out["user_id"] = b[off]; off += 1
    if flags & 0x10:                                    # measurement status
        if len(b) < off + 2:
            return out, "truncated status"
        st = b[off] | (b[off + 1] << 8); off += 2
        out["status"] = f"0x{st:04X}"
        out["irregular_pulse"] = bool(st & 0x0004)
        out["body_movement"] = bool(st & 0x0001)
        out["cuff_too_loose"] = bool(st & 0x0002)
    if absent:
        out["_absent"] = absent

    # Flag values outside physiological range rather than clamping or dropping.
    implausible = {k: out[k] for k, (lo, hi) in PLAUSIBLE.items()
                   if isinstance(out.get(k), float) and not lo <= out[k] <= hi}
    if implausible:
        out["_implausible"] = implausible

    # Unconsumed bytes mean the flags byte did not describe the payload, so at
    # least one field was read from an offset the device never intended. This
    # was computed and never checked, which reported a desynced frame as a
    # clean decode.
    trailing = len(b) - off
    if trailing:
        out["_trailing_bytes"] = trailing
        return out, f"desync: {trailing} unconsumed bytes for flags 0x{flags:02X}"
    return out, None

# Established before the handlers are defined so `bus` is never None when they
# run. DBusGMainLoop must be installed before SystemBus() or signals never fire.
dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
bus = dbus.SystemBus()
# path -> human label, so an indication can name the characteristic it came
# from. 2A36 Intermediate Cuff Pressure shares 2A35's layout and decodes as a
# measurement with NaN diastolic/MAP; an unlabelled log is one transcription
# away from a mid-inflation pressure reading becoming a "real_5" fixture.
subscribed: dict = {}
HANDLER_FAULTED = False

def dump_gatt():
    """Print the whole GATT tree, including the vendor service.

    BlueZ's on-disk cache (/var/lib/bluetooth/<adapter>/<dev>/attributes) records
    services but not characteristics, so the only way to see what the vendor
    service 233bf000-... actually exposes is to enumerate it live. A writable
    control point there is the one remaining path to a host-triggered
    measurement -- the standard Blood Pressure Profile has none -- and it is
    also where a clock-set characteristic would live, which would make the cuff
    emit real timestamps instead of nothing.
    """
    om = dbus.Interface(bus.get_object(BLUEZ, "/"), OM)
    objs = om.GetManagedObjects()
    print("\n" + "-" * 62, flush=True)
    print("GATT TREE", flush=True)
    for path in sorted(objs):
        ifaces = objs[path]
        if CUFF_PATH_FRAG not in path:
            continue
        if "org.bluez.GattService1" in ifaces:
            svc = ifaces["org.bluez.GattService1"]
            print(f"  service {str(svc.get('UUID',''))}", flush=True)
        elif CHRC in ifaces:
            c = ifaces[CHRC]
            flags = ",".join(str(f) for f in c.get("Flags", []))
            print(f"    char  {str(c.get('UUID',''))}  [{flags}]", flush=True)
    print("-" * 62 + "\n", flush=True)


def try_subscribe():
    """StartNotify on the characteristics we want. Returns True if we now hold
    at least one subscription.

    Reports its outcome rather than returning silently. "already subscribed",
    "found no matching characteristic", and "the path filter matched nothing"
    previously produced byte-identical silence -- and silence is also what a
    correctly-working watcher prints while waiting for a measurement.
    """
    matched = skipped = 0
    om = dbus.Interface(bus.get_object(BLUEZ, "/"), OM)
    for path, ifaces in om.GetManagedObjects().items():
        if CHRC not in ifaces or CUFF_PATH_FRAG not in path:
            continue
        uuid = str(ifaces[CHRC].get("UUID", "")).lower()
        if uuid not in WANT:
            continue
        matched += 1
        if path in subscribed:
            skipped += 1
            continue
        flags = [str(f) for f in ifaces[CHRC].get("Flags", [])]
        try:
            dbus.Interface(bus.get_object(BLUEZ, path), CHRC).StartNotify()
            subscribed[path] = WANT[uuid]
            print(f">>> SUBSCRIBED {WANT[uuid]}  flags={flags}", flush=True)
        except dbus.DBusException as e:
            print(f"!!! StartNotify failed on {WANT[uuid]}: {e}", flush=True)
    print(f">>> try_subscribe: {matched} matched, {len(subscribed)} subscribed, "
          f"{skipped} already", flush=True)
    return bool(subscribed)


def retry_subscribe():
    """Re-attempt until something sticks. StartNotify failing transiently
    (InProgress / NotConnected) inside a 5-second radio window is precisely the
    case this tool exists to survive, and it was the one path that logged and
    gave up."""
    if subscribed:
        return False                        # stop the timer
    print("!!! NOT SUBSCRIBED — retrying", flush=True)
    try:
        try_subscribe()
    except dbus.DBusException as e:
        print(f"!!! retry failed: {e}", flush=True)
    return not subscribed                   # keep firing until subscribed

def on_props(*args, **kwargs):
    """Outermost guard. D-Bus swallows every exception a handler raises and
    keeps the mainloop spinning, so an unguarded fault here produces a watcher
    that is running, connected, and permanently deaf -- with nothing in the log
    saying so. The previous version guarded only `path is None`, which the
    signal receiver makes impossible, while four genuinely-raising calls inside
    the same function were unguarded.
    """
    global HANDLER_FAULTED
    try:
        _on_props(*args, **kwargs)
    except BaseException:                                   # noqa: BLE001
        HANDLER_FAULTED = True
        print("!!! HANDLER FAULTED — capture may be dead:", flush=True)
        print(traceback.format_exc(), flush=True)


def _on_props(iface, changed, _invalidated, path=None):
    if not path:
        return
    if iface == DEV and CUFF_PATH_FRAG in path:
        if "Connected" in changed:
            print(f">>> Connected = {bool(changed['Connected'])}", flush=True)
            if not changed["Connected"]:
                subscribed.clear()
        # BlueZ emits ServicesResolved=False on its own when a link drops, and a
        # disconnect does not always carry a Connected key. Clearing only on
        # Connected left stale paths cached, so the next connection skipped
        # StartNotify silently and captured nothing.
        if "ServicesResolved" in changed and not changed["ServicesResolved"]:
            subscribed.clear()
        if changed.get("ServicesResolved"):
            print(">>> ServicesResolved -- subscribing", flush=True)
            # Subscribe FIRST. dump_gatt() is a diagnostic and it makes
            # unguarded D-Bus calls; a NoReply or timeout there would abort the
            # handler before the subscription that is this tool's entire
            # purpose. The cuff's radio window is 5-10s and then it disconnects
            # with the reading still stored, so one transient fault costs a
            # measurement. Diagnostics never run ahead of the functional path.
            ok = False
            try:
                ok = try_subscribe()
            except Exception as e:                      # noqa: BLE001
                print(f"!!! subscribe raised: {e!r}", flush=True)
            if not ok:
                # Arm a retry rather than logging and giving up. ServicesResolved
                # does not re-fire for an already-resolved device, so without
                # this there is no second chance for the whole run.
                GLib.timeout_add_seconds(1, retry_subscribe)
            try:
                dump_gatt()
            except Exception as e:                      # noqa: BLE001
                print(f"!!! dump_gatt raised (non-fatal): {e!r}", flush=True)
    if iface == CHRC and "Value" in changed and CUFF_PATH_FRAG in path:
        raw = bytes(bytearray(changed["Value"]))
        # Name the source characteristic. 2A36 frames decode as a measurement
        # with NaN diastolic/MAP -- plausible enough to be mistaken for a real
        # reading when transcribed into a fixture file.
        src = subscribed.get(path, f"UNKNOWN {path}")
        dec, err = decode_2a35(raw)

        # DURABLE FIRST, print second. The kernel already ACKed this frame and
        # the cuff has already deleted it; a BrokenPipeError or ENOSPC on the
        # first print would otherwise destroy it with nothing left to recover.
        # Raw hex goes in regardless of decode outcome -- the bytes are the
        # irreplaceable part, the interpretation can be redone.
        global CAPTURED
        CAPTURED += 1
        persist({"seq": CAPTURED, "wall_clock": time.time(), "source": src,
                 "hex": raw.hex(), "decoded": dec, "error": err})

        print("\n" + "=" * 62, flush=True)
        print(f"INDICATION from {src}  --  {len(raw)} bytes  [saved to {LOG_PATH}]", flush=True)
        print(f"  hex: {raw.hex()}", flush=True)
        print(f"  arr: [{', '.join(f'0x{x:02X}' for x in raw)}]", flush=True)
        if err:
            print(f"  DECODE ERROR: {err}", flush=True)
        # Print whatever decoded even on error: three good vitals are still
        # worth showing when the fourth field was truncated.
        for k, v in (dec or {}).items():
            print(f"  {k:>18}: {v}", flush=True)
        print("=" * 62 + "\n", flush=True)

# bus and the main loop are established above, before the handlers are defined.
# bus_name scopes the receiver to BlueZ; without it every PropertiesChanged on
# the system bus (systemd, logind, UPower) entered the handler and exercised its
# exception surface with traffic this tool never considered.
bus.add_signal_receiver(on_props, dbus_interface=PROPS,
                        signal_name="PropertiesChanged",
                        bus_name=BLUEZ,
                        path_keyword="path")

TIMEOUT = int(os.environ.get("BP_CAPTURE_TIMEOUT", "0"))
loop = GLib.MainLoop()


def heartbeat():
    """A faulted handler must be loud. Without this, a watcher that went deaf
    is indistinguishable from one waiting patiently for a measurement."""
    if HANDLER_FAULTED:
        print("!!! handler has faulted — capture is NOT running", flush=True)
    elif not subscribed:
        print("!!! still not subscribed", flush=True)
    return True


GLib.timeout_add_seconds(30, heartbeat)
if TIMEOUT:
    GLib.timeout_add_seconds(TIMEOUT, loop.quit)

print(f">>> watching {CUFF} -- kernel auto-connect will bring the link up.")
print(f">>> durable log: {LOG_PATH}")
print(">>> TAKE A MEASUREMENT NOW. Ctrl-C"
      + (f" or {TIMEOUT}s timeout to stop." if TIMEOUT
         else " to stop (set BP_CAPTURE_TIMEOUT for a deadline)."), flush=True)
try_subscribe()   # in case it is already connected
try:
    loop.run()
except KeyboardInterrupt:
    pass
finally:
    LOG.close()

# Exit code carries the outcome. A run that captured four irreplaceable
# measurements and one that captured none were previously indistinguishable to
# any wrapper or systemd unit -- and T-D6 step 6 makes this shape Restart=always.
print(f">>> captured {CAPTURED} frame(s) to {LOG_PATH}", flush=True)
sys.exit(0 if CAPTURED and not HANDLER_FAULTED else 1)
