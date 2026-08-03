/* GENERATED FROM registry.toml — DO NOT EDIT */

import Foundation

public enum VitalsType: Int32 {
    case HR = 1
    case SPO2 = 2
    case ACCEL = 3
    case HRV_RMSSD = 4
    case RR_INTERVAL = 5
    case BpSystolic = 6
    case BpDiastolic = 7
    case BpMap = 8
}

public enum VitalsSource: Int32 {
    case EMAY = 0
    case POLAR_H10 = 1
    case HEALTHKIT = 2
    case DERIVED = 3
    case AD_UA651BLE = 6
}

