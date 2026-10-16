//! `proptest` strategies for the property tests (`tests/properties.rs`).
//!
//! Each strategy builds plain data (amounts, sides, account codes, indices) and maps it onto
//! engine inputs at the end, so a failing case shrinks to few, small lines. Ids that only exist
//! once books are built (entries to reverse, drafts to update) are drawn as a
//! [`proptest::sample::Index`] and resolved against the books when the case runs. Strategy and
//! configuration API: <https://docs.rs/proptest/1.12.0/proptest/>.

use ledger_core::IssueCode;
use ledger_core::Minor;
use ledger_core::types::{EntryInput, LineInput};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::{Index, select};
use proptest::test_runner::Config;

use super::books::{U, acc, entry, line};

/// The cases each property runs: `PROPTEST_CASES` when set, else 1000.
pub fn config() -> Config {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    Config {
        cases,
        ..Config::default()
    }
}

/// A minor amount as the JS number an input carries.
pub fn js(x: Minor) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a JS number input, as TS holds it"
    )]
    let n = x as f64;
    n
}

/// `POSTABLE` (`engine.test.ts:303`): leaf accounts of every type in the template.
pub const POSTABLE: [&str; 12] = [
    "1110", "1120", "1130", "1150", "2100", "4100", "4200", "5100", "5500", "6100", "7100", "7300",
];

/// Which side of a line carries the amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// A debit.
    Debit,
    /// A credit.
    Credit,
}

impl Side {
    /// The amount signed debit-positive.
    pub fn signed(self, amount: Minor) -> Minor {
        match self {
            Self::Debit => amount,
            Self::Credit => -amount,
        }
    }
}

/// A debit or a credit.
pub fn side() -> impl Strategy<Value = Side> {
    prop_oneof![Just(Side::Debit), Just(Side::Credit)]
}

/// A base-currency line of `amount` on `side`.
pub fn sided(account: &str, fund: &str, amount: Minor, side: Side) -> LineInput {
    match side {
        Side::Debit => line(account, fund, amount, 0),
        Side::Credit => line(account, fund, 0, amount),
    }
}

/// `YYYY-MM-DD` from parts.
pub fn ymd(year: u32, month: u32, day: u32) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}

/// A day of 2026 every month has: day 1 to 28 of month 1 to 12, as `balancedArb` draws them
/// (`engine.test.ts:313`).
pub fn date_2026() -> impl Strategy<Value = String> {
    (1..=12u32, 1..=28u32).prop_map(|(month, day)| ymd(2026, month, day))
}

/// A date Bun's `Date` accepts in `month_lo..=month_hi` of 2026: month 1 to 12 and day 1 to 31,
/// where a day past the month's end rolls into the next month (`2026-02-30` is 2 March). Month
/// 0 or 13 and day 0 or 32 are invalid in Bun and are not drawn.
pub fn bun_date(month_lo: u32, month_hi: u32) -> impl Strategy<Value = String> {
    (month_lo..=month_hi, 1..=31u32).prop_map(|(month, day)| ymd(2026, month, day))
}

/// `lineArb` (`engine.test.ts:305-309`): a postable account, 1 to 50,000,000 minor units and a
/// side.
pub fn line_spec() -> impl Strategy<Value = (&'static str, Minor, Side)> {
    (select(&POSTABLE[..]), 1..=50_000_000i64, side())
}

/// `balancedArb` lines (`engine.test.ts:314-319`) in `fund` from drawn specs: the lines, then a
/// plug on `plug` for the net, or a 1/1 pair when they already net to zero.
pub fn balance_lines(specs: &[(&str, Minor, Side)], plug: &str, fund: &str) -> Vec<LineInput> {
    let net: Minor = specs.iter().map(|(_, a, s)| s.signed(*a)).sum();
    let mut out: Vec<LineInput> = specs
        .iter()
        .map(|(account, amount, s)| sided(account, fund, *amount, *s))
        .collect();
    match net.cmp(&0) {
        std::cmp::Ordering::Equal => {
            let other = if plug == POSTABLE[0] {
                POSTABLE[1]
            } else {
                POSTABLE[0]
            };
            out.push(line(plug, fund, 1, 0));
            out.push(line(other, fund, 0, 1));
        }
        std::cmp::Ordering::Greater => out.push(line(plug, fund, 0, net)),
        std::cmp::Ordering::Less => out.push(line(plug, fund, -net, 0)),
    }
    out
}

/// `balancedArb` (`engine.test.ts:311-320`): a random balanced entry in the unrestricted fund,
/// dated in 2026.
pub fn balanced_entry() -> impl Strategy<Value = EntryInput> {
    (vec(line_spec(), 1..=6), select(&POSTABLE[..]), date_2026())
        .prop_map(|(specs, plug, date)| entry(&date, "Generated", balance_lines(&specs, plug, U)))
}

/// The skew of `engine.test.ts:343-346`: `skew` more on the first line's debit if it has one,
/// else on its credit.
pub fn skewed(mut input: EntryInput, skew: Minor) -> EntryInput {
    let target = &mut input.lines[0];
    match target.debit {
        Some(d) if d != 0.0 => target.debit = Some(d + js(skew)),
        _ => target.credit = Some(target.credit.unwrap_or(0.0) + js(skew)),
    }
    input
}

/// One way to break a balanced entry, and the code `validateEntry` (`engine.ts:295-445`) or the
/// permission check (`engine.ts:154-163`) answers it with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Defect {
    /// Debits and credits differ by this much (`engine.ts:404-408`).
    Unbalanced(Minor),
    /// A line amount below zero (`engine.ts:350-351`).
    Negative,
    /// A line with neither side (`engine.ts:354-355`).
    Zero,
    /// A line with both sides (`engine.ts:352-353`).
    BothSides,
    /// A line amount with a fraction of a cent (`engine.ts:348-349`).
    Fractional,
    /// A line on an account that does not exist (`engine.ts:357-358`).
    UnknownAccount,
    /// A line on a header account (`engine.ts:359-360`).
    HeaderAccount,
    /// A line in a fund that does not exist (`engine.ts:364-365`).
    UnknownFund,
    /// A single line (`engine.ts:330-331`).
    OneLine,
    /// A date no period covers, in this year; no books here open 2019 or 2031
    /// (`engine.ts:313-315`).
    NoPeriod(u32),
    /// A blank memo (`engine.ts:310-311`).
    BlankMemo,
    /// A date that is not `YYYY-MM-DD` (`engine.ts:308-309`).
    BadDate,
    /// A foreign-currency line without a rate (`engine.ts:371-374`).
    NoRate,
    /// An actor without `can_post` (`engine.ts:154-163`).
    NoPermission,
}

impl Defect {
    /// The issue code the engine must report.
    pub fn code(self) -> IssueCode {
        match self {
            Self::Unbalanced(_) => IssueCode::Unbalanced,
            Self::Negative => IssueCode::NegativeAmount,
            Self::Zero => IssueCode::LineZero,
            Self::BothSides => IssueCode::LineBothSides,
            Self::Fractional => IssueCode::NotMinorUnits,
            Self::UnknownAccount => IssueCode::UnknownAccount,
            Self::HeaderAccount => IssueCode::GroupAccount,
            Self::UnknownFund => IssueCode::UnknownFund,
            Self::OneLine => IssueCode::TooFewLines,
            Self::NoPeriod(_) => IssueCode::NoPeriod,
            Self::BlankMemo | Self::BadDate => IssueCode::InvalidInput,
            Self::NoRate => IssueCode::FxRateRequired,
            Self::NoPermission => IssueCode::Permission,
        }
    }

    /// `input` with the defect on line `at` (wrapped onto the lines).
    pub fn apply(self, mut input: EntryInput, at: Index) -> EntryInput {
        let i = at.index(input.lines.len());
        let l = &mut input.lines[i];
        let amount = l.debit.or(l.credit).unwrap_or(1.0);
        let set = |l: &mut LineInput, x: f64| {
            if l.debit.is_some() {
                l.debit = Some(x);
            } else {
                l.credit = Some(x);
            }
        };
        match self {
            Self::Unbalanced(skew) => return skewed(input, skew),
            Self::Negative => set(l, -amount),
            Self::Zero => set(l, 0.0),
            Self::BothSides => {
                l.debit = Some(amount);
                l.credit = Some(amount);
            }
            Self::Fractional => set(l, amount + 0.5),
            Self::UnknownAccount => l.account_id = acc("9999"),
            Self::HeaderAccount => l.account_id = acc("7000"),
            Self::UnknownFund => l.fund_id = "fund-nope".into(),
            Self::OneLine => input.lines.truncate(1),
            Self::NoPeriod(year) => input.date = format!("{year}{}", &input.date[4..]),
            Self::BlankMemo => " \t ".clone_into(&mut input.memo),
            Self::BadDate => input.date = input.date.replace('-', "/"),
            Self::NoRate => l.currency = Some("USD".to_owned()),
            Self::NoPermission => {}
        }
        input
    }
}

/// Any [`Defect`].
pub fn defect() -> impl Strategy<Value = Defect> {
    prop_oneof![
        (1..=1_000_000i64).prop_map(Defect::Unbalanced),
        Just(Defect::Negative),
        Just(Defect::Zero),
        Just(Defect::BothSides),
        Just(Defect::Fractional),
        Just(Defect::UnknownAccount),
        Just(Defect::HeaderAccount),
        Just(Defect::UnknownFund),
        Just(Defect::OneLine),
        prop_oneof![Just(2019u32), Just(2031u32)].prop_map(Defect::NoPeriod),
        Just(Defect::BlankMemo),
        Just(Defect::BadDate),
        Just(Defect::NoRate),
        Just(Defect::NoPermission),
    ]
}

/// A balanced entry broken one way: the input, the defect and the line it is on.
pub fn invalid_entry() -> impl Strategy<Value = (EntryInput, Defect)> {
    (balanced_entry(), defect(), any::<Index>()).prop_map(|(input, d, at)| (d.apply(input, at), d))
}

/// Rates of the foreign lines: whole, halves that round to even, and rates that leave
/// fractions to round (`money.ts:36-51`).
pub const RATES: [f64; 7] = [129.5, 130.0, 0.5, 1.5, 0.37, 128.25, 1.0001];

/// A line in a foreign currency or, with `None`, in base currency.
pub type FxLine = (&'static str, Minor, Side, Option<(&'static str, f64)>);

/// Lines in KES, USD and EUR at random rates, plugged in KES so they balance in base
/// currency, then skewed on the first line by 0 (half the cases) to 3 units of its currency.
/// A skew can convert to nothing at a small rate, so whether the entry balances is decided by
/// converting each line, as the engine does.
pub fn fx_entry() -> impl Strategy<Value = (Vec<FxLine>, &'static str, Minor, String)> {
    let currency = prop_oneof![
        Just(None),
        (select(&["USD", "EUR"][..]), select(&RATES[..])).prop_map(Some),
    ];
    (
        vec(
            (select(&POSTABLE[..]), 1..=5_000_000i64, side(), currency),
            1..=5,
        ),
        select(&POSTABLE[..]),
        prop_oneof![Just(0i64), 1..=3i64],
        date_2026(),
    )
}

/// The input line of an [`FxLine`].
pub fn fx_line(l: &FxLine, fund: &str) -> LineInput {
    let (account, amount, s, currency) = *l;
    let mut out = sided(account, fund, amount, s);
    if let Some((code, rate)) = currency {
        out.currency = Some(code.to_owned());
        out.fx_rate = Some(rate);
    }
    out
}

/// A balanced group of lines in one of `funds` funds: specs, plug and fund index.
pub type FundGroup = (Vec<(&'static str, Minor, Side)>, &'static str, Index);

/// A transfer that unbalances two funds: amount, the fund debited and the fund credited.
pub type Transfer = (Minor, Index, Index);

/// An entry across funds: one to three groups, each balanced within its fund, and in a quarter
/// of the cases a transfer that debits one fund and credits another, so the entry balances
/// overall but not per fund (unless both picks are the same fund). Accounts come from
/// [`POSTABLE`] and the `extra` accounts the case added.
pub fn fund_entry(
    extra: usize,
) -> impl Strategy<Value = (Vec<FundGroup>, Option<Transfer>, String)> {
    let account = (0..POSTABLE.len() + extra).prop_map(account_at);
    let spec = (account, 1..=1_000_000i64, side());
    (
        vec(
            (vec(spec, 1..=3), select(&POSTABLE[..]), any::<Index>()),
            1..=3,
        ),
        prop_oneof![
            3 => Just(None),
            1 => (1..=100_000i64, any::<Index>(), any::<Index>()).prop_map(Some),
        ],
        date_2026(),
    )
}

/// Codes of the expense accounts a case may add under header 7000 (`7900` to `7909`).
pub const EXTRA_CODES: [&str; 10] = [
    "7900", "7901", "7902", "7903", "7904", "7905", "7906", "7907", "7908", "7909",
];

/// The account code at `i`: [`POSTABLE`], then [`EXTRA_CODES`].
pub fn account_at(i: usize) -> &'static str {
    match i.checked_sub(POSTABLE.len()) {
        None => POSTABLE[i],
        Some(j) => EXTRA_CODES[j % EXTRA_CODES.len()],
    }
}

/// One write of a random command sequence, sent as JSON through `sanitizeCommand` and
/// `applyCommand` (`commands.ts:25-79`). Entry indices pick among the books' entries when the
/// case runs.
#[derive(Debug, Clone)]
pub enum Op {
    /// `entry.post` of a balanced entry.
    Post(EntryInput),
    /// `entry.post` of an entry broken by any input [`Defect`] (not the actor).
    PostInvalid(EntryInput),
    /// `entry.saveDraft`.
    SaveDraft(EntryInput),
    /// `entry.updateDraft` of the picked entry.
    UpdateDraft(Index, EntryInput),
    /// `entry.postDraft` of the picked entry.
    PostDraft(Index),
    /// `entry.deleteDraft` of the picked entry.
    DeleteDraft(Index),
    /// `entry.reverse` of the picked entry, on a date.
    Reverse(Index, String),
    /// `entry.import` of these entries.
    Import(Vec<EntryInput>),
    /// `period.close` of the month, or (`None`) of the first open month, which closing in
    /// order allows.
    Close(Option<u32>),
    /// `period.reopen` of the month, or (`None`) of the last closed or locked month, which
    /// reopening in reverse order allows.
    Reopen(Option<u32>),
    /// `period.lock` of the month, or (`None`) of the first closed month.
    Lock(Option<u32>),
    /// `account.update` setting `is_active` on a postable account.
    Activate(&'static str, bool),
    /// A command `sanitizeCommand` refuses: an unknown type or a payload that does not fit.
    Junk(u8),
}

/// Any [`Op`], weighted toward postings.
pub fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => balanced_entry().prop_map(Op::Post),
        2 => invalid_entry()
            .prop_filter("the sequence posts as admin", |(_, d)| *d != Defect::NoPermission)
            .prop_map(|(input, _)| Op::PostInvalid(input)),
        2 => balanced_entry().prop_map(Op::SaveDraft),
        1 => (any::<Index>(), balanced_entry()).prop_map(|(i, e)| Op::UpdateDraft(i, e)),
        2 => any::<Index>().prop_map(Op::PostDraft),
        1 => any::<Index>().prop_map(Op::DeleteDraft),
        3 => (any::<Index>(), date_2026()).prop_map(|(i, d)| Op::Reverse(i, d)),
        1 => vec(balanced_entry(), 0..=3).prop_map(Op::Import),
        2 => month_or_next().prop_map(Op::Close),
        1 => month_or_next().prop_map(Op::Reopen),
        1 => month_or_next().prop_map(Op::Lock),
        1 => (select(&POSTABLE[..]), any::<bool>()).prop_map(|(c, b)| Op::Activate(c, b)),
        1 => (0..4u8).prop_map(Op::Junk),
    ]
}

/// A month of 2026, or (three times in four) `None` for the month the period rules allow next.
fn month_or_next() -> impl Strategy<Value = Option<u32>> {
    prop_oneof![1 => (1..=12u32).prop_map(Some), 3 => Just(None)]
}

/// One statement row: date, description, reference and amount, from small pools so that keys
/// repeat within and across files.
pub type RowSpec = (String, &'static str, Option<&'static str>, Minor);

/// A statement row with a Bun-valid date in January or February 2026, mostly in the first half
/// of January where the payments of [`payment`] fall.
pub fn statement_row() -> impl Strategy<Value = RowSpec> {
    (
        prop_oneof![
            3 => (1..=17u32).prop_map(|day| ymd(2026, 1, day)),
            1 => bun_date(1, 2),
        ],
        select(&["ATM WITHDRAWAL", "FUEL", "RENT"][..]),
        select(&[None, Some("INV-100"), Some("INV-101")][..]),
        select(&[-10_000i64, -20_000, -30_000, 5_000][..]),
    )
}

/// A payment from the bank account (7100 debited, 1110 credited) in January 2026: amount,
/// day and the reference its memo carries, if any.
pub type PaymentSpec = (Minor, u32, Option<&'static str>);

/// A [`PaymentSpec`] in the first half of January, with amounts the statement rows can match.
pub fn payment() -> impl Strategy<Value = PaymentSpec> {
    (
        select(&[10_000i64, 20_000, 30_000][..]),
        1..=14u32,
        select(&[None, Some("INV-100"), Some("INV-101")][..]),
    )
}

/// A CSV field from characters that need quoting (`,` `"` CR LF) and some that do not.
pub fn csv_field() -> impl Strategy<Value = String> {
    vec(
        select(&['a', 'Z', '0', ' ', ',', '"', '\n', '\r', '\u{e9}', '-'][..]),
        0..=6,
    )
    .prop_map(|cs| cs.into_iter().collect())
}

/// A CSV row with at least one field holding a letter, so `parseCsv` keeps it (it drops rows
/// whose fields are all blank, `csv.ts:4-37`).
pub fn csv_row() -> impl Strategy<Value = Vec<String>> {
    (vec(csv_field(), 0..=3), "[a-z]{1,4}", any::<Index>()).prop_map(|(mut fields, word, at)| {
        let i = at.index(fields.len() + 1);
        fields.insert(i, word);
        fields
    })
}
