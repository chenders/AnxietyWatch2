#!/usr/bin/env python3
"""Tests for the FR24 poller, the operator roster, and the loud-aircraft log.

Every assertion here exists because of something measured against the live API
on 2026-08-11/12, or because of a failure mode the vendor's own example code
still has. Nothing in this file touches the network.

The station fixture is derived from PDX rather than from the real address, for
the same reason as `test_panel_adsb_status.py`: the home location lives only in
a systemd drop-in on the Pi and must never enter git.
"""

import importlib.machinery
import json
import pathlib
import tempfile
import time
import unittest

_HERE = pathlib.Path(__file__).parent
poll = importlib.machinery.SourceFileLoader(
    "panel_fr24_poll", str(_HERE / "panel-fr24-poll")).load_module()
import panel_loud_operators as ops  # noqa: E402
import panel_noise_log as noise_log  # noqa: E402

# PDX terminal, and a station 6.45 km due south of it -- the same derivation the
# ADS-B tests use, so both suites describe the same geometry without either
# committing an address.
PDX_LAT, PDX_LON = 45.5887, -122.5975
STATION_LAT = PDX_LAT - 6450.0 / 111132.0
STATION_LON = PDX_LON

TOLERANCE = 1e-6


class Geometry(unittest.TestCase):
    def test_haversine_matches_known_separation(self):
        # The station is 6.45 km from PDX by construction.
        km = poll.haversine_km(STATION_LAT, STATION_LON, PDX_LAT, PDX_LON)
        self.assertLess(abs(km - 6.45), 0.01)

    def test_zero_distance_is_zero(self):
        km = poll.haversine_km(STATION_LAT, STATION_LON, STATION_LAT, STATION_LON)
        self.assertLess(abs(km), TOLERANCE)

    def test_bounds_order_is_north_south_west_east(self):
        # Bounds order is a documented trap: the API takes N,S,W,E, and the
        # sibling project had it as tl_y,tl_x,br_y,br_x in one service.
        north, south, west, east = (float(v) for v in
                                    poll.bounds_for(STATION_LAT, STATION_LON, 12).split(","))
        self.assertGreater(north, south)
        self.assertLess(west, east)

    def test_bounds_enclose_the_circle(self):
        # Every point at exactly the radius, in each cardinal direction, must lie
        # inside the box -- otherwise the coarse filter silently clips the circle
        # the model believes it is watching.
        radius = 12.0
        north, south, west, east = (float(v) for v in
                                    poll.bounds_for(STATION_LAT, STATION_LON, radius).split(","))
        self.assertGreaterEqual(north, STATION_LAT + radius / 111.32 - TOLERANCE)
        self.assertLessEqual(south, STATION_LAT - radius / 111.32 + TOLERANCE)
        self.assertLessEqual(west, STATION_LON - 0.05)
        self.assertGreaterEqual(east, STATION_LON + 0.05)


class OperatorIdentification(unittest.TestCase):
    def test_roster_hex_is_confirmed_and_returns_the_airframe(self):
        # N406LF, the Portland-based Bell 407GXi. Its Mode S address is what the
        # local dongle broadcasts, so this path costs no credits at all.
        state, reason, airframe = ops.identify(hex_code="A4C3EB")
        self.assertEqual(state, ops.CONFIRMED)
        self.assertEqual(airframe["reg"], "N406LF")
        self.assertEqual(airframe["type"], "B407")
        self.assertIn("Life Flight Network", reason)

    def test_hex_match_is_case_insensitive(self):
        state, _, airframe = ops.identify(hex_code="a4c3eb")
        self.assertEqual(state, ops.CONFIRMED)
        self.assertEqual(airframe["reg"], "N406LF")

    def test_callsign_pattern_confirms_a_tail_not_on_the_roster(self):
        # A Life Flight tail nobody has added yet must still be recognised the
        # first time it flies.
        state, reason, airframe = ops.identify(callsign="N999LF")
        self.assertEqual(state, ops.CONFIRMED)
        self.assertIsNone(airframe)
        self.assertIn("N999LF", reason)

    def test_fleet_type_alone_is_never_confirmation(self):
        # Other operators fly the EC35. Type is a hint, never an identity.
        state, reason, _ = ops.identify(aircraft_type="EC35")
        self.assertEqual(state, ops.UNCONFIRMED)
        self.assertIn("also by other operators", reason)

    def test_unknown_aircraft_is_unconfirmed_with_a_named_reason(self):
        state, reason, _ = ops.identify(hex_code="ABCDEF", callsign="SKW3441",
                                        aircraft_type="E175")
        self.assertEqual(state, ops.UNCONFIRMED)
        self.assertTrue(reason)

    def test_nothing_reported_says_so(self):
        state, reason, _ = ops.identify()
        self.assertEqual(state, ops.UNCONFIRMED)
        self.assertIn("no Mode S address", reason)

    def test_tisb_address_never_matches_a_real_airframe(self):
        # dump1090 prefixes non-ICAO (TIS-B / ADS-R) addresses with '~'. Matching
        # one against the roster would attribute a synthetic track to a real
        # helicopter.
        self.assertIsNone(ops.normalise_hex("~A4C3EB"))
        state, _, airframe = ops.identify(hex_code="~A4C3EB")
        self.assertEqual(state, ops.UNCONFIRMED)
        self.assertIsNone(airframe)

    def test_malformed_hex_is_rejected(self):
        for bad in ("", "ZZZZZZ", "A4C3E", "A4C3EBB", None):
            self.assertIsNone(ops.normalise_hex(bad))

    def test_portland_police_airframes_are_identified_by_hex(self):
        # Both are fixed-wing Cessna 182T. The Mode S address is the only stable
        # identifier -- the flight ID is pilot-entered and may be blank.
        for hex_code, reg in (("A1D296", "N2163J"), ("A50E9F", "N425EJ")):
            state, _, airframe = ops.identify(hex_code=hex_code)
            self.assertEqual(state, ops.CONFIRMED)
            self.assertEqual(airframe["reg"], reg)
            self.assertEqual(ops.operator_kind(airframe), ops.POLICE)

    def test_police_are_not_matched_on_a_rumoured_callsign(self):
        # "IDAHO44" is a single unverified spotter claim. Filtering on it would
        # mis-attribute unrelated traffic; a false CONFIRMED is worse than none.
        state, _, _ = ops.identify(callsign="IDAHO44")
        self.assertEqual(state, ops.UNCONFIRMED)

    def test_surplused_police_aircraft_is_never_attributed_to_the_police(self):
        # N739MR was sold by PPB in 2023 and is now private -- and it was the
        # single most frequent registration in the 14-day Aurora sample. This is
        # the exact false positive a stale roster would produce.
        state, reason, airframe = ops.identify(callsign="N739MR")
        self.assertEqual(state, ops.UNCONFIRMED)
        self.assertIsNone(airframe)
        self.assertIn("2023", reason)

    def test_no_roster_entry_is_a_retired_registration(self):
        for entry in ops.ROSTER.values():
            self.assertNotIn(entry["reg"].upper(), ops.RETIRED_REGISTRATIONS,
                             f"{entry['reg']} is retired but still on the roster")

    def test_police_fleet_has_no_rotorcraft(self):
        # PPB operates no helicopters; a rotorcraft orbiting Portland is another
        # agency and must not be attributed here.
        police = [e for e in ops.ROSTER.values() if e["operator"] == "PORTLAND_POLICE"]
        self.assertTrue(police)
        for entry in police:
            self.assertNotIn(entry["type"], {"A119", "B407", "EC35", "EC45", "H125"})

    def test_every_roster_entry_names_a_real_operator(self):
        for hex_code, entry in ops.ROSTER.items():
            self.assertEqual(ops.normalise_hex(hex_code), hex_code,
                             f"{hex_code} is not normalised uppercase 6-hex")
            self.assertIn(entry["operator"], ops.OPERATORS)
            self.assertIsNotNone(ops.operator_kind(entry))


class ReportNormalisation(unittest.TestCase):
    def test_record_without_position_is_dropped(self):
        # 0,0 is in the Gulf of Guinea. Defaulting a missing coordinate would put
        # a phantom aircraft 12,000 km away into a proximity calculation.
        self.assertIsNone(poll.to_report({"hex": "A4C3EB"}, STATION_LAT, STATION_LON))
        self.assertIsNone(poll.to_report({"lat": 45.5, "lon": None}, STATION_LAT, STATION_LON))

    def test_roster_fills_in_registration_the_api_withheld(self):
        # FR24 returns reg/type as null for these tails on several endpoints.
        # The Mode S address is enough to recover both locally.
        report = poll.to_report(
            {"hex": "A4C3EB", "lat": STATION_LAT, "lon": STATION_LON, "reg": None, "type": None},
            STATION_LAT, STATION_LON)
        self.assertEqual(report["reg"], "N406LF")
        self.assertEqual(report["type"], "B407")
        self.assertEqual(report["air_ambulance"], ops.CONFIRMED)

    def test_api_values_win_over_the_roster(self):
        report = poll.to_report(
            {"hex": "A4C3EB", "lat": STATION_LAT, "lon": STATION_LON, "reg": "N406LF",
             "type": "B407"},
            STATION_LAT, STATION_LON)
        self.assertEqual(report["reg"], "N406LF")

    def test_absent_callsign_never_becomes_the_string_None(self):
        # str(None) printed "None" as a callsign on the panel once.
        report = poll.to_report({"lat": STATION_LAT, "lon": STATION_LON, "callsign": None},
                                STATION_LAT, STATION_LON)
        self.assertIsNone(report["callsign"])

    def test_thrust_phase_reads_a_hard_climb(self):
        # The measured Providence departure crossed the station at +1088 fpm.
        self.assertEqual(poll.thrust_phase(1088), "CLIMB")
        self.assertEqual(poll.thrust_phase(-500), "DESCENT")
        self.assertEqual(poll.thrust_phase(0), "LEVEL")

    def test_missing_vertical_rate_is_not_level(self):
        # Absent is a third state; folding it into LEVEL would invent a phase.
        self.assertIsNone(poll.thrust_phase(None))


class NotableSelection(unittest.TestCase):
    def test_confirmed_air_ambulance_is_notable_at_any_range(self):
        far = {"air_ambulance": ops.CONFIRMED, "distance_km": 11.0}
        self.assertTrue(poll.is_notable(far))

    def test_anonymous_aircraft_must_be_close(self):
        self.assertFalse(poll.is_notable({"air_ambulance": ops.UNCONFIRMED, "distance_km": 9.0}))
        self.assertTrue(poll.is_notable({"air_ambulance": ops.UNCONFIRMED, "distance_km": 2.0}))

    def test_missing_distance_is_not_treated_as_close(self):
        self.assertFalse(poll.is_notable({"air_ambulance": ops.UNCONFIRMED, "distance_km": None}))


class CreditAccounting(unittest.TestCase):
    def test_empty_response_costs_one_credit(self):
        # Measured 2026-08-12: five empty polls cost exactly five credits, and
        # the vendor documents "queries that return no data cost 1 credit".
        self.assertEqual(poll.CREDITS_PER_EMPTY_QUERY, 1)

    def test_light_endpoint_costs_six_per_aircraft(self):
        self.assertEqual(poll.CREDITS_PER_AIRCRAFT_LIGHT, 6)

    def test_cycle_rolls_on_the_billing_day_not_the_first(self):
        # Subscription credits expire on the billing day. A ledger keyed on the
        # calendar month would reset weeks late and let a runaway loop spend a
        # whole cycle before the ceiling noticed.
        import calendar
        before = calendar.timegm((2026, 9, 5, 12, 0, 0, 0, 0, 0))
        after = calendar.timegm((2026, 9, 6, 12, 0, 0, 0, 0, 0))
        self.assertEqual(poll.cycle_key(before, 6), "2026-08")
        self.assertEqual(poll.cycle_key(after, 6), "2026-09")

    def test_january_rolls_back_into_the_previous_year(self):
        import calendar
        moment = calendar.timegm((2026, 1, 3, 12, 0, 0, 0, 0, 0))
        self.assertEqual(poll.cycle_key(moment, 6), "2025-12")

    def test_ledger_blocks_with_a_named_blocker_and_a_value(self):
        # "Guards must say why": a guard that blocks without naming the number
        # that blocked it is worse than no guard.
        with tempfile.TemporaryDirectory() as tmp:
            ledger = poll.CreditLedger(pathlib.Path(tmp) / "credits.json", 10, 6)
            ledger.roll(0.0)
            self.assertIsNone(ledger.blocked())
            ledger.charge(10)
            blocker = ledger.blocked()
            self.assertIsNotNone(blocker)
            self.assertIn("10", blocker)
            self.assertIn("ceiling", blocker)

    def test_ledger_survives_a_restart(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "credits.json"
            first = poll.CreditLedger(path, 1000, 6)
            first.roll(0.0)
            first.charge(250)
            second = poll.CreditLedger(path, 1000, 6)
            self.assertEqual(second.spent, 250)

    def test_corrupt_ledger_does_not_crash_the_daemon(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "credits.json"
            path.write_text("{not json")
            ledger = poll.CreditLedger(path, 1000, 6)
            self.assertEqual(ledger.spent, 0)


class FeedState(unittest.TestCase):
    """A dead feed must never look like a clear sky."""

    def _ledger(self, tmp):
        ledger = poll.CreditLedger(pathlib.Path(tmp) / "c.json", 1000, 6)
        ledger.roll(0.0)
        return ledger

    def test_fresh_feed_carries_the_list(self):
        with tempfile.TemporaryDirectory() as tmp:
            doc = poll.build_document(1.0, [], None, self._ledger(tmp), 3.0, None)
            self.assertEqual(doc["feed_state"], "FRESH")
            self.assertEqual(doc["aircraft"], [])
            self.assertIsNone(doc["error"])

    def test_unreachable_api_is_not_an_empty_sky(self):
        # The critical case. An empty list under UNAVAILABLE would read as
        # "nothing overhead" to a consumer that forgot to check feed_state, so
        # the list is None -- a consumer that ignores the state gets a TypeError
        # rather than a false all-clear.
        with tempfile.TemporaryDirectory() as tmp:
            doc = poll.build_document(1.0, None, "FR24 unreachable: timed out",
                                      self._ledger(tmp), 3.0, None)
            self.assertEqual(doc["feed_state"], "UNAVAILABLE")
            self.assertIsNone(doc["aircraft"])
            self.assertIn("unreachable", doc["error"])

    def test_credit_block_is_its_own_state_and_names_itself(self):
        with tempfile.TemporaryDirectory() as tmp:
            doc = poll.build_document(1.0, None, None, self._ledger(tmp), 3.0,
                                      "credit ceiling reached: 600000 of 600000")
            self.assertEqual(doc["feed_state"], "BLOCKED")
            self.assertIsNone(doc["aircraft"])
            self.assertIn("600000", doc["error"])

    def test_document_is_json_serialisable_without_nan(self):
        with tempfile.TemporaryDirectory() as tmp:
            doc = poll.build_document(1.0, [], None, self._ledger(tmp), 3.0, None)
            json.dumps(doc, allow_nan=False)


class EdgeRefusalIsNotAuthFailure(unittest.TestCase):
    """A 403 from the CDN must never be reported as a bad credential.

    Measured 2026-08-12: every request from this daemon returned 403 while the
    identical URL and key returned 200 from curl. The cause was urllib's default
    User-Agent being rejected at the edge. Diagnosing that as an auth failure
    would send the next reader to rotate a perfectly good key.
    """

    class _FakeError:
        """Just enough of HTTPError for `_http_error`.

        Deliberately not a subclass: constructing a real HTTPError leaves an
        unclosed file object and emits a ResourceWarning, and `_http_error`
        only ever touches `.code`, `.headers` and `.read()`.
        """

        def __init__(self, code, content_type, body):
            self.code = code
            self.headers = {"Content-Type": content_type}
            self._body = body

        def read(self, n=None):
            return self._body[:n] if n is not None else self._body

    def _client(self):
        return poll.Fr24Client("token")

    def test_user_agent_is_not_the_urllib_default(self):
        self.assertNotIn("Python-urllib", poll.USER_AGENT)
        self.assertTrue(poll.USER_AGENT.strip())

    def test_html_403_is_named_as_an_edge_block_and_warns_against_rotating(self):
        message = self._client()._http_error(
            self._FakeError(403, "text/html", b"<html>Attention Required</html>"))
        self.assertIn("403", message)
        self.assertIn("NOT a credential problem", message)
        self.assertIn("do not rotate", message)

    def test_json_401_is_reported_as_a_credential_problem(self):
        message = self._client()._http_error(
            self._FakeError(401, "application/json", b'{"message":"Unauthorized"}'))
        self.assertIn("401", message)
        self.assertIn("FR24_API_KEY", message)

    def test_json_402_names_credit_exhaustion(self):
        message = self._client()._http_error(
            self._FakeError(402, "application/json", b'{"message":"Payment required"}'))
        self.assertIn("402", message)
        self.assertIn("credits", message)

    def test_429_sets_a_backoff_window(self):
        client = self._client()
        message = client._http_error(self._FakeError(429, "application/json", b"{}"))
        self.assertIn("rate limit", message.lower())
        self.assertGreater(client.blocked_until, time.time())


class NoiseLogRetention(unittest.TestCase):
    def test_fr24_rows_expire_and_local_rows_do_not(self):
        # The owner has directed six-month retention for FR24-sourced rows; rows
        # from our own receiver carry no expiry because no licence governs them.
        event = noise_log.build_event(at=1000.0, provenance=noise_log.FR24)
        self.assertEqual(event["retain_until"], 1000.0 + noise_log.FR24_RETENTION_SECONDS)
        local = noise_log.build_event(at=1000.0, provenance=noise_log.ADSB)
        self.assertIsNone(local["retain_until"])

    def test_retention_is_at_least_six_months(self):
        self.assertGreaterEqual(noise_log.FR24_RETENTION_SECONDS, 182 * 24 * 3600)

    def test_prune_removes_only_expired_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "log.jsonl"
            noise_log.append(noise_log.build_event(at=0.0, provenance=noise_log.FR24), path)
            noise_log.append(noise_log.build_event(at=0.0, provenance=noise_log.ADSB), path)
            later = noise_log.FR24_RETENTION_SECONDS + 1
            self.assertEqual(noise_log.prune(now=later, path=path), 1)
            rows = noise_log.read_all(path)
            self.assertEqual(len(rows), 1)
            self.assertEqual(rows[0]["provenance"], noise_log.ADSB)

    def test_prune_is_a_no_op_when_nothing_expired(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "log.jsonl"
            noise_log.append(noise_log.build_event(at=0.0, provenance=noise_log.FR24), path)
            self.assertEqual(noise_log.prune(now=1.0, path=path), 0)
            self.assertEqual(len(noise_log.read_all(path)), 1)

    def test_truncated_final_row_does_not_discard_the_file(self):
        # The expected outcome of a power cut mid-append.
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "log.jsonl"
            noise_log.append(noise_log.build_event(at=1.0, provenance=noise_log.ADSB), path)
            with path.open("a") as stream:
                stream.write('{"at": 2.0, "prov')
            rows = noise_log.read_all(path)
            self.assertEqual(len(rows), 1)

    def test_missing_file_reads_as_empty_not_an_error(self):
        self.assertEqual(noise_log.read_all(pathlib.Path("/nonexistent/log.jsonl")), [])


class NoiseLogContent(unittest.TestCase):
    def test_unscored_aircraft_is_never_recorded_as_quiet(self):
        # Absence of a score is not evidence of quiet. A 0 or "FAINT" here would
        # make the most important row in the log read as the least important.
        event = noise_log.build_event(at=1.0, provenance=noise_log.FR24)
        self.assertIsNone(event["noise_index"])
        self.assertEqual(event["loudness_band"], "UNKNOWN")

    def test_flight_link_uses_the_aircraft_page_with_an_id_fragment(self):
        url = noise_log.flight_url("N406LF", "40934418")
        self.assertEqual(url, "https://www.flightradar24.com/data/aircraft/n406lf#40934418")

    def test_links_are_none_rather_than_broken_when_identity_is_withheld(self):
        # FR24 withholds reg for these tails on several endpoints. A link built
        # from a missing field would 404, which in an admin console reads as
        # "not recorded" rather than "not named".
        self.assertIsNone(noise_log.flight_url(None, "40934418"))
        self.assertIsNone(noise_log.flight_url("N406LF", None))
        self.assertIsNone(noise_log.aircraft_url(None))

    def test_event_carries_both_links_when_identity_is_known(self):
        event = noise_log.build_event(at=1.0, provenance=noise_log.FR24,
                                      registration="N406LF", fr24_id="40934418")
        self.assertIn("n406lf#40934418", event["links"]["flight"])
        self.assertIn("data/aircraft/n406lf", event["links"]["aircraft"])


class PassCollapsing(unittest.TestCase):
    """One event per pass, and a returning aircraft is not suppressed."""

    def test_many_polls_of_one_aircraft_yield_one_event(self):
        tracker = noise_log.PassTracker(gap_seconds=120.0)
        for tick in range(20):
            tracker.observe({"fr24_id": "40934418", "distance_km": 5.0 - tick * 0.2}, tick * 3.0)
        self.assertEqual(tracker.close_finished(60.0), [])
        finished = tracker.close_finished(300.0)
        self.assertEqual(len(finished), 1)

    def test_the_closest_observation_is_the_one_reported(self):
        # The closest point is what was loud; the last poll is not.
        tracker = noise_log.PassTracker(gap_seconds=10.0)
        tracker.observe({"fr24_id": "X", "distance_km": 8.0}, 0.0)
        tracker.observe({"fr24_id": "X", "distance_km": 0.95}, 3.0)
        tracker.observe({"fr24_id": "X", "distance_km": 6.0}, 6.0)
        (_, report), = tracker.close_finished(100.0)
        self.assertLess(abs(report["distance_km"] - 0.95), TOLERANCE)

    def test_an_aircraft_that_returns_produces_a_second_event(self):
        # The vendor's geofence example keeps an `alerted` set it never clears,
        # so an aircraft that leaves and comes back is suppressed forever. Here
        # that would silence the Providence *departure* -- the louder half of the
        # visit, climbing at full power over the station.
        tracker = noise_log.PassTracker(gap_seconds=10.0)
        tracker.observe({"fr24_id": "Y", "distance_km": 1.0}, 0.0)
        self.assertEqual(len(tracker.close_finished(100.0)), 1)
        tracker.observe({"fr24_id": "Y", "distance_km": 1.0}, 200.0)
        self.assertEqual(len(tracker.close_finished(400.0)), 1)

    def test_a_single_dropped_poll_does_not_split_one_pass_in_two(self):
        tracker = noise_log.PassTracker(gap_seconds=120.0)
        tracker.observe({"fr24_id": "Z", "distance_km": 2.0}, 0.0)
        tracker.observe({"fr24_id": "Z", "distance_km": 1.5}, 30.0)
        self.assertEqual(tracker.close_finished(60.0), [])

    def test_identity_falls_through_when_fr24_id_is_missing(self):
        # No single identifier is reliable for these aircraft.
        self.assertEqual(noise_log.PassTracker.key_for({"hex": "A4C3EB"}), "hex:A4C3EB")
        self.assertEqual(noise_log.PassTracker.key_for({"callsign": "N406LF"}),
                         "callsign:N406LF")
        self.assertIsNone(noise_log.PassTracker.key_for({}))

    def test_an_unidentifiable_observation_is_ignored_rather_than_merged(self):
        # Without this, every anonymous return would collapse into one pass and
        # the log would report a single aircraft that never left.
        tracker = noise_log.PassTracker(gap_seconds=10.0)
        tracker.observe({"distance_km": 1.0}, 0.0)
        self.assertEqual(tracker.close_finished(100.0), [])


if __name__ == "__main__":
    unittest.main()
