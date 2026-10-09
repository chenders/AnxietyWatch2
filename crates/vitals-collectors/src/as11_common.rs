//! Product-neutral `AirSense` FIG and RPC transport helpers.
//!
//! This module contains no `BlueZ` or D-Bus access. Linux transport ownership is
//! kept in the broker client; onboarding has a separate legacy transport until
//! its own migration is approved.

use vitals_decode::fig::{FIG_MAX_PAYLOAD, FIG_PREFIX_LENGTH, FIG_SYNC};

use crate::{
    as11_crypto::{CryptoError, EnvelopeError},
    as11_rpc::{AIR11_EVENT_IDS, RpcError, RpcResponse, RpcResponseError},
};

/// A bounded `BlueZ` operation failure.  It contains operation names and limits,
/// never a target address, credential, session key, or device payload.
#[derive(Debug)]
pub enum BluezError {
    /// A `BlueZ` or D-Bus operation returned an error.
    Operation {
        /// The operation that failed.
        operation: &'static str,
    },
    /// Another process already owns adapter discovery.
    DiscoveryAlreadyActive,
    /// The configured target was not present in the bounded discovery window.
    TargetNotAdvertised,
    /// A `BlueZ` operation did not complete before its deadline.
    Deadline {
        /// The operation that exceeded its deadline.
        operation: &'static str,
    },
    /// The configured adapter name was invalid or unavailable.
    AdapterUnavailable,
    /// The normalized target could not be converted to a `BlueZ` address.
    AddressInvalid,
    /// The required service was not resolved.
    ServiceMissing,
    /// A required characteristic was not resolved.
    CharacteristicMissing {
        /// The missing characteristic name.
        name: &'static str,
    },
    /// Two matching objects were resolved, which is ambiguous and unsafe.
    DuplicateCharacteristic {
        /// The ambiguous characteristic name.
        name: &'static str,
    },
    /// The TX characteristic does not support a write operation.
    TxNotWritable,
    /// The RX characteristic has no notification or indication capability.
    RxNotNotifiable,
    /// The negotiated MTU cannot carry an ATT value.
    MtuInvalid {
        /// The invalid negotiated MTU.
        mtu: usize,
    },
    /// The notification stream ended before the pending operation completed.
    NotificationEnded,
    /// A bounded FIG notification could not be retained or decoded.
    FigProtocol {
        /// The protocol invariant that failed.
        reason: &'static str,
    },
    /// A decrypted response was not a valid bounded `StreamData` object.
    StreamDataInvalid,
    /// The device returned a JSON-RPC error; only its code is retained.
    DeviceRpc {
        /// The device error code.
        code: i64,
    },
    /// The device response omitted a required authentication field.
    RpcResultInvalid {
        /// The missing result field.
        field: &'static str,
    },
    /// The device accepted the request but did not cover one fixed selector.
    EventSubscriptionCoverage {
        /// The selector whose coverage was absent or invalid.
        selector: &'static str,
        /// The device-reported validity, or false when it was absent.
        valid: bool,
    },
    /// The typed request failed the final read-only gate.
    RpcRejected(RpcError),
    /// The final transport queue or GATT write failed.
    Transport(TransportError),
    /// The correlated response was malformed or arrived on the wrong lane.
    Response(RpcResponseError),
    /// The broker rejected or could not complete a typed operation.
    Broker(vitals_ble_broker::BrokerClientError),
    /// Portable crypto rejected a configured or received value.
    Crypto(CryptoError),
    /// Portable envelope encryption/decryption failed.
    Envelope(EnvelopeError),
    /// The request-id counter reached its fixed integer ceiling.
    RequestIdExhausted,
}

impl core::fmt::Display for BluezError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Operation { operation } => write!(f, "bluez operation failed: {operation}"),
            Self::DiscoveryAlreadyActive => {
                f.write_str("bluez discovery already active; target scan blocked")
            }
            Self::TargetNotAdvertised => {
                f.write_str("air11 target not advertised during bounded discovery")
            }
            Self::Deadline { operation } => write!(f, "bluez deadline expired: {operation}"),
            Self::AdapterUnavailable => f.write_str("bluez adapter unavailable"),
            Self::AddressInvalid => f.write_str("bluez target address invalid"),
            Self::ServiceMissing => f.write_str("air11 service fd56 not resolved"),
            Self::CharacteristicMissing { name } => {
                write!(f, "air11 characteristic missing: {name}")
            }
            Self::DuplicateCharacteristic { name } => {
                write!(f, "air11 characteristic ambiguous: {name}")
            }
            Self::TxNotWritable => f.write_str("air11 TX characteristic is not writable"),
            Self::RxNotNotifiable => f.write_str("air11 RX characteristic is not notifiable"),
            Self::MtuInvalid { mtu } => write!(f, "air11 negotiated MTU={mtu} is too small"),
            Self::NotificationEnded => f.write_str("air11 notification stream ended"),
            Self::FigProtocol { reason } => write!(f, "air11 FIG protocol error: {reason}"),
            Self::StreamDataInvalid => f.write_str("air11 StreamData payload invalid"),
            Self::DeviceRpc { code } => write!(f, "air11 device RPC error code={code}"),
            Self::RpcResultInvalid { field } => {
                write!(f, "air11 RPC result missing field={field}")
            }
            Self::EventSubscriptionCoverage { selector, valid } => write!(
                f,
                "air11 event subscription selector={selector} valid={valid}"
            ),
            Self::RpcRejected(error) => write!(f, "air11 RPC rejected: {error}"),
            Self::Broker(error) => write!(f, "air11 broker: {error}"),
            Self::Transport(error) => write!(f, "air11 transport: {error}"),
            Self::Response(error) => write!(f, "air11 response: {error}"),
            Self::Crypto(error) => write!(f, "air11 crypto: {error}"),
            Self::Envelope(error) => write!(f, "air11 envelope: {error}"),
            Self::RequestIdExhausted => f.write_str("air11 RPC request id exhausted"),
        }
    }
}

impl std::error::Error for BluezError {}

/// Failure of the final queued GATT writer.  Payloads are intentionally not
/// retained in any variant.
#[derive(Debug)]
pub enum TransportError {
    /// The crypto mutex was poisoned after a task failure.
    CryptoStatePoisoned,
    /// A request attempted to use an unsupported FIG lane.
    InvalidVcid {
        /// The rejected virtual circuit id.
        vcid: u16,
    },
    /// The FIG payload exceeded its fixed protocol capacity.
    FigPayloadTooLarge {
        /// Observed payload length.
        len: usize,
        /// Maximum accepted payload length.
        max: usize,
    },
    /// Another request is still waiting for its GATT write acknowledgement.
    WritePending,
    /// The bounded writer queue is full.
    WriteQueueFull,
    /// The writer task has no receiver.
    WriterStopped,
    /// The waiter state was poisoned.
    WaiterStatePoisoned,
    /// The caller attempted to await a write that was not reserved.
    WriteNotPending,
    /// The characteristic write failed.
    GattWriteFailed,
    /// The broker rejected or could not complete a typed write.
    BrokerWriteFailed {
        /// Named broker blocker without the GATT payload.
        detail: String,
    },
    /// The characteristic write exceeded its deadline.
    GattWriteDeadline,
    /// The MTU leaves no room for an ATT value.
    MtuTooSmall {
        /// The rejected MTU.
        mtu: usize,
    },
    /// The random-IV envelope could not be produced.
    Envelope(EnvelopeError),
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CryptoStatePoisoned => f.write_str("crypto state poisoned"),
            Self::InvalidVcid { vcid } => write!(f, "unsupported request VCID=0x{vcid:04x}"),
            Self::FigPayloadTooLarge { len, max } => {
                write!(f, "FIG payload length={len} exceeds limit={max}")
            }
            Self::WritePending => f.write_str("a GATT write is already pending"),
            Self::WriteQueueFull => f.write_str("GATT write queue is full"),
            Self::WriterStopped => f.write_str("GATT writer stopped"),
            Self::WaiterStatePoisoned => f.write_str("GATT waiter state poisoned"),
            Self::WriteNotPending => f.write_str("no GATT write acknowledgement is pending"),
            Self::GattWriteFailed => f.write_str("GATT write failed"),
            Self::BrokerWriteFailed { detail } => {
                write!(f, "broker GATT write failed: {detail}")
            }
            Self::GattWriteDeadline => f.write_str("GATT write deadline expired"),
            Self::MtuTooSmall { mtu } => write!(f, "negotiated MTU={mtu} is too small"),
            Self::Envelope(error) => write!(f, "envelope encryption failed: {error}"),
        }
    }
}

impl std::error::Error for TransportError {}

pub(crate) fn encode_fig(vcid: u16, payload: &[u8]) -> Result<Vec<u8>, TransportError> {
    if payload.len() > FIG_MAX_PAYLOAD || payload.len() > usize::from(u16::MAX) {
        return Err(TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        });
    }
    let payload_length =
        u16::try_from(payload.len()).map_err(|_| TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        })?;
    let payload_crc = crc32_ieee(payload);
    let mut header = [0_u8; 8];
    header
        .get_mut(..2)
        .ok_or(TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        })?
        .copy_from_slice(&vcid.to_le_bytes());
    header
        .get_mut(2..4)
        .ok_or(TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        })?
        .copy_from_slice(&payload_length.to_le_bytes());
    header
        .get_mut(4..8)
        .ok_or(TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        })?
        .copy_from_slice(&payload_crc.to_le_bytes());
    let header_crc = crc32_ieee(&header);
    let mut packet = Vec::with_capacity(FIG_PREFIX_LENGTH + payload.len());
    packet.extend_from_slice(&FIG_SYNC);
    packet.extend_from_slice(&header);
    packet.extend_from_slice(&header_crc.to_le_bytes());
    packet.extend_from_slice(payload);
    Ok(packet)
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

pub(crate) fn result_string(
    response: &RpcResponse,
    field: &'static str,
) -> Result<String, BluezError> {
    let result = response.result().ok_or_else(|| device_error(response))?;
    result
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or(BluezError::RpcResultInvalid { field })
}

pub(crate) fn ensure_success(response: &RpcResponse) -> Result<(), BluezError> {
    if response.result().is_some() {
        Ok(())
    } else {
        Err(device_error(response))
    }
}

pub(crate) fn ensure_event_subscription(response: &RpcResponse) -> Result<(), BluezError> {
    let result = response.result().ok_or_else(|| device_error(response))?;
    let object = result.as_object().ok_or(BluezError::RpcResultInvalid {
        field: "event_subscription_result",
    })?;
    if object
        .get("subscriptionId")
        .and_then(serde_json::Value::as_u64)
        .is_none_or(|value| value == 0)
    {
        return Err(BluezError::RpcResultInvalid {
            field: "event_subscription_id",
        });
    }
    let data_ids = object
        .get("dataIds")
        .and_then(serde_json::Value::as_array)
        .ok_or(BluezError::RpcResultInvalid {
            field: "event_subscription_dataIds",
        })?;
    for selector in AIR11_EVENT_IDS {
        let Some(item) = data_ids
            .iter()
            .find(|item| item.get("dataId").and_then(serde_json::Value::as_str) == Some(selector))
        else {
            return Err(BluezError::EventSubscriptionCoverage {
                selector,
                valid: false,
            });
        };
        let valid = item
            .get("valid")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if !valid {
            return Err(BluezError::EventSubscriptionCoverage { selector, valid });
        }
    }
    Ok(())
}

pub(crate) fn ensure_confirmation(response: &RpcResponse) -> Result<(), BluezError> {
    let result = response.result().ok_or_else(|| device_error(response))?;
    if result
        .get("confirmation")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        Ok(())
    } else {
        Err(BluezError::RpcResultInvalid {
            field: "confirmation",
        })
    }
}

pub(crate) fn device_error(response: &RpcResponse) -> BluezError {
    response
        .device_error()
        .map_or(BluezError::RpcResultInvalid { field: "result" }, |error| {
            BluezError::DeviceRpc { code: error.code }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vitals_decode::fig::decode_fig;

    #[test]
    fn encoded_fig_round_trips_through_the_hostile_decoder() -> Result<(), &'static str> {
        let packet = encode_fig(0x0102, br#"{"id":1}"#).map_err(|_| "FIG")?;
        let decoded = decode_fig(&packet).ok_or("decoded FIG")?;
        assert_eq!(decoded.vcid, 0x0102);
        assert_eq!(decoded.payload, br#"{"id":1}"#);
        Ok(())
    }

    #[test]
    fn oversized_fig_payload_is_rejected_before_allocation() {
        let payload = vec![0_u8; FIG_MAX_PAYLOAD + 1];
        assert!(matches!(
            encode_fig(0x0102, &payload),
            Err(TransportError::FigPayloadTooLarge { .. })
        ));
    }

    #[test]
    fn broker_write_error_names_the_underlying_blocker() {
        let error = TransportError::BrokerWriteFailed {
            detail: "broker response blocked: expected=OperationAccepted".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "broker GATT write failed: broker response blocked: expected=OperationAccepted"
        );
    }

    #[test]
    fn event_subscription_requires_every_selector_to_be_valid() {
        let result = serde_json::json!({
            "subscriptionId": 4,
            "dataIds": AIR11_EVENT_IDS.iter().enumerate().map(|(index, selector)| {
                serde_json::json!({"dataId": selector, "valid": index != 4})
            }).collect::<Vec<_>>()
        });
        let response = RpcResponse::Result { id: 1, result };
        assert!(matches!(
            ensure_event_subscription(&response),
            Err(BluezError::EventSubscriptionCoverage {
                selector: "TherapyEvents-RespiratoryEvents",
                valid: false
            })
        ));
    }

    #[test]
    fn event_subscription_accepts_complete_coverage() {
        let result = serde_json::json!({
            "subscriptionId": 4,
            "dataIds": AIR11_EVENT_IDS.iter().map(|selector| {
                serde_json::json!({"dataId": selector, "valid": true})
            }).collect::<Vec<_>>()
        });
        let response = RpcResponse::Result { id: 1, result };
        assert!(ensure_event_subscription(&response).is_ok());
    }

    #[test]
    fn response_helpers_preserve_read_only_success_and_rejection() {
        let response = RpcResponse::Result {
            id: 1,
            result: serde_json::json!({
                "confirmation": true,
                "value": "ready"
            }),
        };
        assert!(ensure_success(&response).is_ok());
        assert!(ensure_confirmation(&response).is_ok());
        assert_eq!(
            result_string(&response, "value").ok().as_deref(),
            Some("ready")
        );

        let rejected = RpcResponse::Error {
            id: 1,
            error: crate::as11_rpc::RpcDeviceError {
                code: 403,
                message: "rejected".to_owned(),
                data: None,
            },
        };
        assert!(matches!(
            ensure_success(&rejected),
            Err(BluezError::DeviceRpc { code: 403 })
        ));
        assert!(matches!(ensure_confirmation(&response), Ok(())));
    }
}
