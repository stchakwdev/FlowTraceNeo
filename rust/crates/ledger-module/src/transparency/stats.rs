//! Campaign figures (`getCampaignStats`, `flowtrace/src/lib/demo/api.ts:1130-1165`) and the
//! instants they need.
//!
//! The module keeps the figures that only depend on the campaign and its donations in the
//! `campaign_stat` table. `days_remaining` and `days_active` depend on the day they are read, so
//! readers compute them with [`days_remaining`] and [`days_active`].

use ledger_core::money::js_round;
use ledger_core::types::Minor;

use super::types::{Campaign, CampaignStat, Donation, PaymentStatus};

/// `DAY` (`api.ts:1128`): one day in milliseconds.
pub const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Milliseconds since the Unix epoch of a UTC instant as the TS writes them: `YYYY-MM-DD`,
/// `YYYY-MM-DDTHH:MM:SSZ` or `YYYY-MM-DDTHH:MM:SS.sssZ` (`toISOString`, and the demo dataset's
/// `ts()` at `flowtrace/src/lib/demo/dataset.ts:29-30`). `None` for anything else, where
/// `new Date(s).getTime()` would be `NaN` or depend on the local time zone.
#[must_use]
pub fn iso_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> {
        let part = s.get(from..from + len)?;
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    let year = num(0, 4)?;
    let month = num(5, 2)?;
    let day = num(8, 2)?;
    if b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return None;
    }
    let (hour, minute, second, millis) = match b.len() {
        10 => (0, 0, 0, 0),
        20 | 24 => {
            let shape = b[10] == b'T' && b[13] == b':' && b[16] == b':' && b[b.len() - 1] == b'Z';
            let millis = if b.len() == 24 {
                if b[19] != b'.' {
                    return None;
                }
                num(20, 3)?
            } else {
                0
            };
            if !shape {
                return None;
            }
            (num(11, 2)?, num(14, 2)?, num(17, 2)?, millis)
        }
        _ => return None,
    };
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(
        days_from_civil(year, month, day) * DAY_MS
            + hour * 3_600_000
            + minute * 60_000
            + second * 1000
            + millis,
    )
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant, `days_from_civil`:
/// <https://howardhinnant.github.io/date_algorithms.html#days_from_civil>).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The donor a donation counts for in `unique_donors` (`api.ts:1138-1142`): an anonymous gift is
/// its own donor, else the email, else the name, else the gift.
fn donor_key(d: &Donation) -> &str {
    if d.is_anonymous {
        &d.id
    } else {
        d.donor_email
            .as_deref()
            .or(d.donor_name.as_deref())
            .unwrap_or(&d.id)
    }
}

/// A campaign's figures from its confirmed donations (`getCampaignStats`, `api.ts:1130-1165`),
/// in minor units. `avg_donation` is rounded to the minor unit where the TS, holding major units,
/// rounds to the major unit (`rust/fixtures/README.md`, formatting notes).
#[must_use]
pub fn campaign_stat(campaign: &Campaign, donations: &[Donation]) -> CampaignStat {
    let confirmed: Vec<&Donation> = donations
        .iter()
        .filter(|d| d.campaign_id == campaign.id && d.payment_status == PaymentStatus::Confirmed)
        .collect();
    let amounts: Vec<Minor> = confirmed.iter().map(|d| d.amount).collect();
    let mut donors: Vec<&str> = confirmed.iter().map(|d| donor_key(d)).collect();
    donors.sort_unstable();
    donors.dedup();
    let raised: Minor = amounts.iter().sum();
    let count = amounts.len();
    #[expect(
        clippy::cast_precision_loss,
        reason = "amounts are safe integers and counts small; JS does this division in doubles"
    )]
    let (funding_percentage, avg_donation) = (
        if campaign.target_amount > 0 {
            js_round(raised as f64 / campaign.target_amount as f64 * 1000.0) / 10.0
        } else {
            0.0
        },
        if count == 0 {
            0
        } else {
            to_minor(js_round(raised as f64 / count as f64))
        },
    );
    CampaignStat {
        campaign_id: campaign.id.clone(),
        donation_count: count as u64,
        unique_donors: donors.len() as u64,
        total_raised: raised,
        funding_percentage,
        avg_donation,
        largest_donation: amounts.iter().copied().max().unwrap_or(0),
        smallest_donation: amounts.iter().copied().min().unwrap_or(0),
    }
}

/// A rounded double that holds a safe integer, as an integer.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is a rounded mean of safe integers, so it is a safe integer"
)]
fn to_minor(x: f64) -> Minor {
    x as Minor
}

/// `days_remaining` (`api.ts:1157-1159`): whole days, rounded up, from `now_ms` to the campaign's
/// end, never below zero; `None` for an open-ended campaign or an end the module cannot read.
#[must_use]
pub fn days_remaining(ends_at: Option<&str>, now_ms: i64) -> Option<i64> {
    let end = iso_ms(ends_at?)?;
    Some(0.max(ceil_div(end - now_ms, DAY_MS)))
}

/// `days_active` (`api.ts:1160-1163`): whole days, rounded down, since the campaign started,
/// never below zero; zero for a start the module cannot read.
#[must_use]
pub fn days_active(starts_at: &str, now_ms: i64) -> i64 {
    iso_ms(starts_at).map_or(0, |start| 0.max((now_ms - start).div_euclid(DAY_MS)))
}

/// `Math.ceil(a / b)` for a positive `b`.
const fn ceil_div(a: i64, b: i64) -> i64 {
    -(-a).div_euclid(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_ms_reads_the_three_shapes_the_ts_writes() {
        let day = iso_ms("2026-02-04").unwrap_or_default();
        assert_eq!(
            (
                iso_ms("2026-02-04T09:00:00Z"),
                iso_ms("2026-02-04T09:00:00.250Z")
            ),
            (Some(day + 9 * 3_600_000), Some(day + 9 * 3_600_000 + 250))
        );
    }

    #[test]
    fn iso_ms_knows_the_epoch_and_leap_days() {
        assert_eq!(
            (
                iso_ms("1970-01-01"),
                iso_ms("2024-02-29"),
                iso_ms("2025-02-29")
            ),
            (Some(0), Some(1_709_164_800_000), None)
        );
    }

    #[test]
    fn iso_ms_refuses_local_times_and_junk() {
        assert_eq!(
            (
                iso_ms("2026-02-04T09:00:00"),
                iso_ms("04/02/2026"),
                iso_ms("2026-13-01")
            ),
            (None, None, None)
        );
    }

    #[test]
    fn days_remaining_rounds_up_and_stops_at_zero() {
        let now = iso_ms("2026-02-04T09:00:00.000Z").unwrap_or_default();
        assert_eq!(
            (
                days_remaining(Some("2026-02-05T08:00:00Z"), now),
                days_remaining(Some("2026-01-01T00:00:00Z"), now),
                days_remaining(None, now)
            ),
            (Some(1), Some(0), None)
        );
    }

    #[test]
    fn days_active_rounds_down_and_is_zero_before_the_start() {
        let now = iso_ms("2026-02-04T09:00:00.000Z").unwrap_or_default();
        assert_eq!(
            (
                days_active("2026-02-03T10:00:00Z", now),
                days_active("2026-03-01T00:00:00Z", now)
            ),
            (0, 0)
        );
    }
}
