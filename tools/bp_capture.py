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
import sys
import dbus
import dbus.mainloop.glib
from gi.repository import GLib

BLUEZ, OM = "org.bluez", "org.freedesktop.DBus.ObjectManager"
PROPS, CHRC, DEV = ("org.freedesktop.DBus.Properties",
                    "org.bluez.GattCharacteristic1", "org.bluez.Device1")

BPM = "00002a35-0000-1000-8000-00805f9b34fb"   # Blood Pressure Measurement
ICP = "00002a36-0000-1000-8000-00805f9b34fb"   # Intermediate Cuff Pressure
WANT = {BPM: "2A35 BloodPressureMeasurement", ICP: "2A36 IntermediateCuffPressure"}

CUFF = sys.argv[1] if len(sys.argv) > 1 else "00:09:1F:98:EB:CF"
CUFF_PATH_FRAG = "dev_" + CUFF.replace(":", "_")

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
    """Returns (dict, error). Every length check is required: radio input."""
    if len(b) < 7:
        return None, f"too short: {len(b)} bytes, need >= 7"
    out, flags, off = {}, b[0], 1
    out["_flags"] = f"0x{flags:02X}"
    out["units"] = "kPa" if flags & 0x01 else "mmHg"
    for name in ("systolic", "diastolic", "map"):
        val, special = sfloat(b[off] | (b[off + 1] << 8)); off += 2
        out[name] = val if special is None else f"<{special}>"
    if flags & 0x02:                                    # timestamp, 7 bytes
        if len(b) < off + 7:
            return None, "truncated timestamp"
        yr = b[off] | (b[off + 1] << 8)
        out["timestamp"] = (f"{yr:04d}-{b[off+2]:02d}-{b[off+3]:02d} "
                            f"{b[off+4]:02d}:{b[off+5]:02d}:{b[off+6]:02d}")
        off += 7
    if flags & 0x04:                                    # pulse rate, SFLOAT
        if len(b) < off + 2:
            return None, "truncated pulse"
        val, special = sfloat(b[off] | (b[off + 1] << 8)); off += 2
        out["pulse_bpm"] = val if special is None else f"<{special}>"
    if flags & 0x08:                                    # user id
        if len(b) < off + 1:
            return None, "truncated user id"
        out["user_id"] = b[off]; off += 1
    if flags & 0x10:                                    # measurement status
        if len(b) < off + 2:
            return None, "truncated status"
        st = b[off] | (b[off + 1] << 8); off += 2
        out["status"] = f"0x{st:04X}"
        out["irregular_pulse"] = bool(st & 0x0004)
        out["body_movement"] = bool(st & 0x0001)
        out["cuff_too_loose"] = bool(st & 0x0002)
    out["_trailing_bytes"] = len(b) - off
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
    """Find the cuff's characteristics and StartNotify on the ones we want."""
    om = dbus.Interface(bus.get_object(BLUEZ, "/"), OM)
    for path, ifaces in om.GetManagedObjects().items():
        if CHRC not in ifaces or CUFF_PATH_FRAG not in path:
            continue
        uuid = str(ifaces[CHRC].get("UUID", "")).lower()
        if uuid not in WANT or path in subscribed:
            continue
        flags = [str(f) for f in ifaces[CHRC].get("Flags", [])]
        try:
            dbus.Interface(bus.get_object(BLUEZ, path), CHRC).StartNotify()
            subscribed[path] = WANT[uuid]
            print(f">>> SUBSCRIBED {WANT[uuid]}  flags={flags}", flush=True)
        except dbus.DBusException as e:
            print(f"!!! StartNotify failed on {WANT[uuid]}: {e}", flush=True)

def on_props(iface, changed, _invalidated, path=None):
    # path is keyword-supplied by the signal receiver and is always a str here,
    # but guard anyway: a None slipping through would raise inside the handler
    # and D-Bus swallows handler exceptions, which is how a watcher goes silent
    # while still looking alive.
    if not path:
        return
    if iface == DEV and CUFF_PATH_FRAG in path:
        if "Connected" in changed:
            print(f">>> Connected = {bool(changed['Connected'])}", flush=True)
            if not changed["Connected"]:
                subscribed.clear()
        if changed.get("ServicesResolved"):
            print(">>> ServicesResolved -- subscribing", flush=True)
            # Subscribe FIRST. dump_gatt() is a diagnostic and it makes
            # unguarded D-Bus calls; a NoReply or timeout there would abort the
            # handler before the subscription that is this tool's entire
            # purpose. The cuff's radio window is 5-10s and then it disconnects
            # with the reading still stored, so one transient fault costs a
            # measurement. Diagnostics never run ahead of the functional path.
            try:
                try_subscribe()
            except Exception as e:                      # noqa: BLE001
                print(f"!!! subscribe raised: {e!r}", flush=True)
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
        print("\n" + "=" * 62, flush=True)
        print(f"INDICATION from {src}  --  {len(raw)} bytes", flush=True)
        print(f"  hex: {raw.hex()}", flush=True)
        print(f"  arr: [{', '.join(f'0x{x:02X}' for x in raw)}]", flush=True)
        dec, err = decode_2a35(raw)
        if err or dec is None:
            print(f"  DECODE ERROR: {err}", flush=True)
        else:
            for k, v in dec.items():
                print(f"  {k:>18}: {v}", flush=True)
        print("=" * 62 + "\n", flush=True)

# bus and the main loop are established above, before the handlers are defined.
bus.add_signal_receiver(on_props, dbus_interface=PROPS,
                        signal_name="PropertiesChanged",
                        path_keyword="path")
print(f">>> watching {CUFF} -- kernel auto-connect will bring the link up.")
print(">>> TAKE A MEASUREMENT NOW. Ctrl-C or timeout to stop.", flush=True)
try_subscribe()   # in case it is already connected
GLib.MainLoop().run()
