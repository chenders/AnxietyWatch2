# How AnxietyWatch2 calculates your personal baseline

*A clinician-friendly walkthrough of the rolling baseline used in trend charts, alerting, and PDF reports.*

---

## What is "the baseline"?

When the trends chart shows a dashed horizontal line on your HRV graph, or the CNS-depression klaxon evaluates your SpO₂ drop, it's referring to a **personal rolling average** — *your* numbers over the recent past, not population norms.

This matters because the same number can be normal for one person and concerning for another. A resting heart rate of 70 bpm is fine for most adults, but for someone whose 30-day average is 55, it's a meaningful 27% elevation.

The baseline is computed for each metric independently:

| Metric | Source |
|---|---|
| HRV (SDNN) | Polar H10 or Apple Watch |
| Resting HR | Polar H10 or Apple Watch |
| Sleep duration | Apple Watch |
| Respiratory rate | Apple Watch |
| CPAP AHI | ResMed AirSense data (hms-cpap) |
| SpO₂ nadir (overnight low) | EMAY oximeter |
| T90 (minutes <90% SpO₂) | EMAY oximeter |

Each metric has its own baseline because the inputs vary in shape. These values are processed by `vitals-core` Spool running in the AnxietyWatch2 Raspberry Pi backend.

## The four-step calculation

### Step 1: Pick a window

We use the **last 30 days**. Older data is excluded.
When the alarm engine evaluates a realtime `vitalsd` stream, it compares against the baseline computed up to the *start* of the current session to avoid self-contamination (a desat should not pull its own baseline down).

### Step 2: Require enough data

If the window has fewer than **14 valid samples**, no baseline is computed. The engine falls back to absolute thresholds (e.g. absolute SpO₂ ≤ 80%).

### Step 3: Trim outliers using MAD

Real-world health data has outliers — a 200 bpm heart rate when you sprinted up the stairs, a 30 ms HRV during a panic episode. If we took a simple average, outliers would pull the baseline toward extremes.

We use **median absolute deviation (MAD)** to identify outliers in the `vitals-core` implementation:

1. Compute the median of the window.
2. For each sample, compute `|sample - median|`.
3. The median of those deviations is the MAD.
4. Trim any sample where `|sample - median| > 2.5 × MAD × 1.4826`.

The `1.4826` scaling factor makes MAD roughly equivalent to a standard deviation under a normal distribution, but it is **robust** (resistant to large outliers).

### Step 4: Compute bounds

From the trimmed values, `vitals-core` computes the **Mean**, **Standard deviation**, and **Lower/Upper bounds**.

For HRV, a value below the lower bound is the clinically meaningful direction. For resting HR, the upper bound is what matters. The alerting engine factors these bounds into the klaxon logic.

## A worked example

1. **Window**: the most recent 30 samples.
2. **Median**: 42 ms.
3. **MAD**: 6 ms. Trim threshold = `2.5 × 6 × 1.4826` = 22.2 ms.
   - Any value below `42 - 22.2 = 19.8` ms or above `42 + 22.2 = 64.2` ms is dropped.
4. **Trimmed mean**: 41 ms. **Stddev**: 7 ms.
   - Lower bound: 30.5 ms.
   - Upper bound: 51.5 ms.

## What this is not

- **Not a medical diagnosis.** A reading outside the baseline band means "different from your recent average," not "abnormal" in any clinical sense.
- **Not a population norm.** Two people with identical health can have very different baselines.
- **Not stable forever.** As underlying health changes, the baseline shifts to follow.
