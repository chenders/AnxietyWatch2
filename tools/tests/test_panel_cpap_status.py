from tools.panel_cpap_status import latest_values


def test_latest_values_maps_live_air11_fields() -> None:
    result = latest_values(
        {
            "status": {"therapy_state": "active", "blocker": None},
            "latest": [
                {"name": "respiratory_rate", "value": 18.0, "t_micros": 1_700_000_100_000_000},
                {"name": "leak", "value": 2.5, "t_micros": 1_700_000_101_000_000},
                {"name": "mask_pressure", "value": 9.8, "t_micros": 1_700_000_101_000_000},
            ],
        }
    )
    assert result["on"] is True
    assert result["respiratory_rate"] == 18.0
    assert result["leak_lpm"] == 2.5
    assert result["pressure_cmh2o"] == 9.8
    assert result["sample_at"] == 1_700_000_101.0


def test_latest_values_preserves_off_state_and_absent_measurements() -> None:
    result = latest_values(
        {"status": {"therapy_state": "inactive", "blocker": "not streaming"}, "latest": []}
    )
    assert result["on"] is False
    assert result["respiratory_rate"] is None
    assert result["sample_at"] is None
    assert result["blocker"] == "not streaming"
