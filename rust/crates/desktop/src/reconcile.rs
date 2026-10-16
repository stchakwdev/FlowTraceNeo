//! Reconcile screen view models, free of GPUI: the bank and mobile-money accounts with their
//! book and statement figures, the add-account form, the statement import preview, each
//! statement line's match status, the candidate journal lines of an open line, the position of
//! an account against its statement, and the `bank.add` and `statement.*` commands.
//!
//! Web screens mirrored: `flowtrace/src/routes/_authenticated/reconcile/index.tsx` (the accounts)
//! and `reconcile/$bankId.tsx` (one account). The matching itself is the engine's:
//! `ledger_core::reconcile::{parse_statement, candidates_for}` and the `statement.autoMatch`
//! command (`flowtrace/src/lib/ledger/reconcile.ts:41-166`).

use ledger_core::LedgerCommand;
use ledger_core::Minor;
use ledger_core::commands::{NewBankAccount, StatementCounter};
use ledger_core::reconcile::{
    AutoMatchResult, DEFAULT_WINDOW_DAYS, ParsedStatementRow, StatementFormat,
};
use ledger_core::types::{
    AccountId, BankAccountId, BankAccountKind, EntryId, FundId, GrantId, IsoDate, LedgerState,
    LineId, StatementLine, StatementLineId,
};

/// The window auto-match uses (`$bankId.tsx:77`, `reconcile.ts:130`).
pub const AUTO_WINDOW: u32 = DEFAULT_WINDOW_DAYS;

/// Manual matching looks a little wider (`$bankId.tsx:76-78`).
pub const MANUAL_WINDOW: u32 = 7;

/// `KIND_LABEL` (`index.tsx:25`).
#[must_use]
pub fn kind_label(kind: BankAccountKind) -> &'static str {
    let _ = kind;
    ""
}

/// `FORMAT_LABEL` (`$bankId.tsx:69-74`).
#[must_use]
pub fn format_label(format: StatementFormat) -> &'static str {
    let _ = format;
    ""
}

/// `formatMinor(minor, { signed: true })` (`flowtrace/src/components/ledger/money.tsx:8-21`):
/// a minus sign (U+2212) instead of parentheses, zero as an en dash.
#[must_use]
pub fn format_signed(minor: Minor) -> String {
    let _ = minor;
    String::new()
}

/// One bank or mobile-money account in the list (`index.tsx:30-47`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankRow {
    /// The account.
    pub id: BankAccountId,
    /// Its name.
    pub name: String,
    /// Bank, Mobile money or Cash.
    pub kind: &'static str,
    /// Provider.
    pub provider: String,
    /// Masked number.
    pub number_masked: String,
    /// Account currency.
    pub currency: String,
    /// The linked GL account as `code name`, or `Cash account` when it is not in the chart.
    pub gl: String,
    /// The GL account's book balance at the as-of date.
    pub ledger_balance: Minor,
    /// The net of every statement line imported.
    pub statement_balance: Minor,
    /// Statement lines.
    pub lines: usize,
    /// Of those, matched.
    pub matched: usize,
    /// Of those, unmatched.
    pub unmatched: usize,
    /// The latest statement date.
    pub last: Option<IsoDate>,
}

impl BankRow {
    /// The coverage label (`index.tsx:124-133`): `88% of 8 statement lines matched`, or
    /// `No statement imported yet`.
    #[must_use]
    pub fn coverage_text(&self) -> String {
        String::new()
    }
}

/// The accounts list (`index.tsx:30-47`), in the books' order.
#[must_use]
pub fn bank_rows(state: &LedgerState, as_of: &IsoDate) -> Vec<BankRow> {
    let _ = (state, as_of);
    Vec::new()
}

/// The key figures across all accounts (`index.tsx:49-91`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overview {
    /// Cash movement reconciled by value, as `NN%` (`ledgerSignals().reconciledShare`).
    pub reconciled: String,
    /// Accounts.
    pub accounts: usize,
    /// Statement lines.
    pub lines: usize,
    /// Unmatched statement lines.
    pub unmatched: usize,
}

/// The overview of `rows` on `state` at `as_of`.
#[must_use]
pub fn overview(state: &LedgerState, as_of: &IsoDate, rows: &[BankRow]) -> Overview {
    let _ = (state, as_of, rows);
    Overview {
        reconciled: String::new(),
        accounts: 0,
        lines: 0,
        unmatched: 0,
    }
}

/// The add-account form (`bank.add`). The web has no such form; the fields are the engine's
/// `NewBankAccount` (`flowtrace/src/lib/ledger/engine.ts:1144-1151`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBankForm {
    /// Display name.
    pub name: String,
    /// Bank, mobile money or cash.
    pub kind: BankAccountKind,
    /// Provider.
    pub provider: String,
    /// The linked GL cash account.
    pub account: Option<AccountId>,
    /// Currency.
    pub currency: String,
    /// Masked number.
    pub number_masked: String,
}

impl NewBankForm {
    /// An empty form in the books' base currency.
    #[must_use]
    pub fn new(state: &LedgerState) -> Self {
        Self {
            name: String::new(),
            kind: BankAccountKind::Bank,
            provider: String::new(),
            account: None,
            currency: state.settings.base_currency.clone(),
            number_masked: String::new(),
        }
    }

    /// The `bank.add` command, or the first thing missing. The engine checks the account is a
    /// cash account of the chart.
    ///
    /// # Errors
    ///
    /// A message when the name or the account is missing.
    pub fn command(&self) -> Result<LedgerCommand, String> {
        Err(String::new())
    }
}

/// The import preview (`$bankId.tsx:510-713`): what `parse_statement` read, what it skipped,
/// the money in and out, and how many rows the engine will leave out as already imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPreview {
    /// The detected format.
    pub format: StatementFormat,
    /// The rows read.
    pub rows: Vec<ParsedStatementRow>,
    /// Each skipped row as `Line n: reason`.
    pub skipped: Vec<String>,
    /// Money in.
    pub money_in: Minor,
    /// Money out (negative).
    pub money_out: Minor,
    /// Rows already on the account's statement.
    pub repeats: usize,
}

impl ImportPreview {
    /// The preview of `text` for `bank`, `None` for a blank text.
    #[must_use]
    pub fn of(state: &LedgerState, bank: &BankAccountId, text: &str) -> Option<Self> {
        let _ = (state, bank, text);
        None
    }

    /// Rows the import will add.
    #[must_use]
    pub fn to_add(&self) -> usize {
        0
    }

    /// The Import button: `Import 3 lines`, or `Import` when nothing was read.
    #[must_use]
    pub fn button_label(&self) -> String {
        String::new()
    }

    /// The `statement.import` command, `None` when no row was read.
    #[must_use]
    pub fn command(&self, bank: &BankAccountId) -> Option<LedgerCommand> {
        let _ = bank;
        None
    }
}

/// How many of `rows` `importStatement` leaves out as already on `bank`'s statement
/// (`flowtrace/src/lib/ledger/engine.ts:1166-1170`).
#[must_use]
pub fn repeats(state: &LedgerState, bank: &BankAccountId, rows: &[ParsedStatementRow]) -> usize {
    let _ = (state, bank, rows);
    0
}

/// The notice after an import (`$bankId.tsx:313-320`).
#[must_use]
pub fn import_notice(added: usize, total: usize) -> String {
    let _ = (added, total);
    String::new()
}

/// A statement line's state (`MatchState`, `$bankId.tsx:80-83`), with Posted split out of
/// manual matches: a line matched to the entry posted from it (`source_type` statement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineStatus {
    /// Matched by auto-match.
    Auto,
    /// Matched by hand to an existing entry.
    Manual,
    /// Posted as a new entry from the line, and matched to it.
    Posted,
    /// Unmatched, with more than one candidate.
    Ambiguous,
    /// Unmatched.
    Unmatched,
}

/// The status filters, `None` for all lines.
pub const STATUSES: [Option<LineStatus>; 6] = [
    None,
    Some(LineStatus::Auto),
    Some(LineStatus::Manual),
    Some(LineStatus::Posted),
    Some(LineStatus::Ambiguous),
    Some(LineStatus::Unmatched),
];

impl LineStatus {
    /// `MATCH_LABEL` (`flowtrace/src/components/ledger/reports/match-mark.tsx:9-14`).
    #[must_use]
    pub fn label(self) -> &'static str {
        ""
    }

    /// `MATCH_SHORT` (`match-mark.tsx:16-21`).
    #[must_use]
    pub fn short(self) -> &'static str {
        ""
    }

    /// Matched in any way.
    #[must_use]
    pub fn matched(self) -> bool {
        false
    }
}

/// `matchState` (`$bankId.tsx:80-83`) with Posted.
#[must_use]
pub fn status_of(state: &LedgerState, line: &StatementLine) -> LineStatus {
    let _ = (state, line);
    LineStatus::Unmatched
}

/// One statement line on the account page (`$bankId.tsx:96-105`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRow {
    /// The statement line.
    pub id: StatementLineId,
    /// Value date.
    pub date: IsoDate,
    /// Statement text.
    pub description: String,
    /// Statement reference.
    pub reference: Option<String>,
    /// Signed amount.
    pub amount: Minor,
    /// Its state.
    pub status: LineStatus,
    /// The entry of the matched journal line, as id and number.
    pub entry: Option<(EntryId, String)>,
}

/// The statement lines of `bank`, latest first, ties in import order (`$bankId.tsx:96-105`).
#[must_use]
pub fn line_rows(state: &LedgerState, bank: &BankAccountId) -> Vec<LineRow> {
    let _ = (state, bank);
    Vec::new()
}

/// The lines in each state (`$bankId.tsx:107-111`).
#[must_use]
pub fn count(rows: &[LineRow], status: Option<LineStatus>) -> usize {
    let _ = (rows, status);
    0
}

/// One candidate journal line for an open statement line (`MatchDialog`, `$bankId.tsx:719-800`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRow {
    /// The journal line.
    pub journal_line: LineId,
    /// Its entry.
    pub entry: EntryId,
    /// The entry number.
    pub number: String,
    /// The entry date.
    pub date: IsoDate,
    /// The line memo, or else the entry memo.
    pub memo: String,
    /// Days between the entry and the statement line.
    pub day_gap: i64,
    /// The statement reference appears in the entry.
    pub reference_match: bool,
}

impl CandidateRow {
    /// `Same day`, `1 day apart`, `n days apart`, then ` · Reference matches` when it does.
    #[must_use]
    pub fn gap_text(&self) -> String {
        String::new()
    }
}

/// The candidates of statement line `line` within `window` days, in the engine's order:
/// reference matches first, then by day gap, ties in line order (`candidates_for`).
#[must_use]
pub fn candidates(state: &LedgerState, line: &StatementLineId, window: u32) -> Vec<CandidateRow> {
    let _ = (state, line, window);
    Vec::new()
}

/// The account against its statement (`Summary`, `$bankId.tsx:386-504`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// The GL account's book balance at the as-of date.
    pub book_balance: Minor,
    /// First and last statement dates.
    pub span: Option<(IsoDate, IsoDate)>,
    /// Net of the matched statement lines.
    pub matched: Minor,
    /// Net of the open statement lines.
    pub only_statement: Minor,
    /// Net of the posted, unmatched, non-opening book lines in the statement's dates.
    pub only_books: Minor,
    /// How many such book lines.
    pub only_books_count: usize,
}

impl Position {
    /// Statement net movement.
    #[must_use]
    pub fn statement_net(&self) -> Minor {
        0
    }

    /// Books net movement.
    #[must_use]
    pub fn books_net(&self) -> Minor {
        0
    }

    /// Statement less books.
    #[must_use]
    pub fn difference(&self) -> Minor {
        0
    }

    /// The sentence under the table (`$bankId.tsx:490-494`).
    #[must_use]
    pub fn verdict(&self) -> &'static str {
        ""
    }
}

/// The position of `bank` at `as_of`.
#[must_use]
pub fn position(state: &LedgerState, bank: &BankAccountId, as_of: &IsoDate) -> Position {
    let _ = (state, bank, as_of);
    Position {
        book_balance: 0,
        span: None,
        matched: 0,
        only_statement: 0,
        only_books: 0,
        only_books_count: 0,
    }
}

/// `AutoMatchReport` (`$bankId.tsx:357-378`): the three lines after an auto-match.
#[must_use]
pub fn auto_match_summary(result: &AutoMatchResult) -> [String; 3] {
    let _ = result;
    [String::new(), String::new(), String::new()]
}

/// `statement.autoMatch` on `bank` (`$bankId.tsx:123-133`).
#[must_use]
pub fn auto_match_command(bank: &BankAccountId) -> LedgerCommand {
    let _ = bank;
    LedgerCommand::StatementUnmatch {
        statement_line_id: StatementLineId::from(""),
    }
}

/// `statement.match` of `line` to `journal_line` (`$bankId.tsx:734-743`).
#[must_use]
pub fn match_command(line: &StatementLineId, journal_line: &LineId) -> LedgerCommand {
    let _ = journal_line;
    LedgerCommand::StatementUnmatch {
        statement_line_id: line.clone(),
    }
}

/// `statement.unmatch` of `line` (`$bankId.tsx:135-147`).
#[must_use]
pub fn unmatch_command(line: &StatementLineId) -> LedgerCommand {
    let _ = line;
    LedgerCommand::StatementUnmatch {
        statement_line_id: StatementLineId::from(""),
    }
}

/// `AccountSelect types=...` of `PostDialog` (`$bankId.tsx:883`): for money out, expense, asset
/// and liability accounts; for money in, income, asset and liability; active, non-header, by
/// type then code (`flowtrace/src/components/ledger/pickers.tsx:25-44`).
#[must_use]
pub fn post_accounts(state: &LedgerState, amount: Minor) -> Vec<(AccountId, String)> {
    let _ = (state, amount);
    Vec::new()
}

/// The Post as a new entry form (`PostDialog`, `$bankId.tsx:806-910`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostForm {
    /// The other side; the suspense account when empty.
    pub account: Option<AccountId>,
    /// The fund of both lines.
    pub fund: Option<FundId>,
    /// The grant of the other line.
    pub grant: Option<GrantId>,
    /// The entry memo.
    pub memo: String,
}

impl PostForm {
    /// The form for `line`: no account, the unrestricted fund (or the first), no grant, the
    /// statement description as memo (`$bankId.tsx:818-821`).
    #[must_use]
    pub fn new(state: &LedgerState, line: &StatementLine) -> Self {
        let _ = (state, line);
        Self {
            account: None,
            fund: None,
            grant: None,
            memo: String::new(),
        }
    }

    /// Choosing a fund clears the grant (`$bankId.tsx:893-896`).
    pub fn set_fund(&mut self, fund: Option<FundId>) {
        self.fund = fund;
    }

    /// The `statement.post` command (`$bankId.tsx:825-839`).
    ///
    /// # Errors
    ///
    /// `Choose the fund this money belongs to.` when no fund is chosen.
    pub fn command(&self, line: &StatementLineId) -> Result<LedgerCommand, String> {
        let _ = line;
        Err(String::new())
    }
}

/// The hint under the account (`$bankId.tsx:875`).
#[must_use]
pub fn suspense_hint(state: &LedgerState) -> String {
    let _ = state;
    String::new()
}

/// Keeps the import of [`StatementCounter`] and [`NewBankAccount`] used until the commands are
/// built.
#[doc(hidden)]
pub fn _types(_: Option<StatementCounter>, _: Option<NewBankAccount>) {}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use ledger_core::CommandResult;
    use ledger_core::coa_template::{UNRESTRICTED_FUND_ID, account_id};
    use ledger_core::demo::{DEMO_AS_OF, build_demo_ledger, demo_actor};
    use ledger_core::env::fixed;
    use ledger_core::types::{BankAccountKind, Nullish};
    use ledger_core::{Ledger, apply_command};

    fn demo() -> LedgerState {
        build_demo_ledger().unwrap()
    }

    fn as_of() -> IsoDate {
        IsoDate::parse(DEMO_AS_OF).unwrap()
    }

    fn bank(id: &str) -> BankAccountId {
        BankAccountId::from(id)
    }

    fn line(id: &str) -> StatementLineId {
        StatementLineId::from(id)
    }

    /// Runs `command` on `state` as the demo user with a test env.
    fn run(state: &mut LedgerState, command: LedgerCommand) -> CommandResult {
        let mut env = fixed::test();
        apply_command(&mut Ledger::new(state, &mut env), &demo_actor(), command).unwrap()
    }

    #[test]
    fn labels_follow_the_web() {
        assert_eq!(kind_label(BankAccountKind::Bank), "Bank");
        assert_eq!(kind_label(BankAccountKind::MobileMoney), "Mobile money");
        assert_eq!(kind_label(BankAccountKind::Cash), "Cash");
        assert_eq!(
            format_label(StatementFormat::Mpesa),
            "Safaricom M-Pesa statement"
        );
        assert_eq!(format_label(StatementFormat::Mtn), "MTN MoMo statement");
        assert_eq!(
            format_label(StatementFormat::Airtel),
            "Airtel Money statement"
        );
        assert_eq!(format_label(StatementFormat::Bank), "Bank CSV");
        assert_eq!(format_signed(-1_230_000), "\u{2212}12,300.00");
        assert_eq!(format_signed(1_500_000), "15,000.00");
        assert_eq!(format_signed(0), "\u{2013}");
        assert_eq!(LineStatus::Auto.label(), "Matched automatically");
        assert_eq!(LineStatus::Manual.short(), "By hand");
        assert_eq!(LineStatus::Ambiguous.label(), "Needs a decision");
        assert_eq!(LineStatus::Posted.label(), "Posted from the statement");
        assert!(LineStatus::Posted.matched() && !LineStatus::Ambiguous.matched());
    }

    #[test]
    fn bank_list_figures_on_the_demo() {
        let state = demo();
        let rows = bank_rows(&state, &as_of());
        let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            ["bank-main", "bank-ops", "bank-projects", "mpesa-till"]
        );
        let main = &rows[0];
        assert_eq!(main.name, "Main operating account");
        assert_eq!(main.kind, "Bank");
        assert_eq!(main.gl, "1110 Bank: Main operating account");
        assert_eq!(main.ledger_balance, 891_875_000);
        assert_eq!(main.statement_balance, 712_216_200);
        assert_eq!((main.lines, main.matched, main.unmatched), (8, 7, 1));
        assert_eq!(main.last, Some(IsoDate::parse("2026-02-01").unwrap()));
        assert_eq!(main.coverage_text(), "88% of 8 statement lines matched");
        let till = &rows[3];
        assert_eq!(till.kind, "Mobile money");
        assert_eq!(till.ledger_balance, 6_270_000);
        assert_eq!(till.statement_balance, 229_000);
        assert_eq!((till.lines, till.matched, till.unmatched), (3, 0, 3));
        let ops = &rows[1];
        assert_eq!((ops.lines, ops.last.clone()), (0, None));
        assert_eq!(ops.coverage_text(), "No statement imported yet");

        let o = overview(&state, &as_of(), &rows);
        assert_eq!((o.accounts, o.lines, o.unmatched), (4, 11, 4));
        assert!(o.reconciled.ends_with('%'), "{}", o.reconciled);
    }

    #[test]
    fn new_bank_form_builds_bank_add() {
        let state = demo();
        let mut form = NewBankForm::new(&state);
        assert_eq!(form.currency, "KES");
        assert_eq!(form.command().unwrap_err(), "Enter the account name.");
        form.name = "  Petty cash  ".to_owned();
        assert_eq!(
            form.command().unwrap_err(),
            "Choose the cash account in the chart it posts to."
        );
        form.account = Some(account_id("1120"));
        form.kind = BankAccountKind::Cash;
        let LedgerCommand::BankAdd { input } = form.command().unwrap() else {
            panic!("not bank.add");
        };
        assert_eq!(input.name, "Petty cash");
        assert_eq!(input.kind, BankAccountKind::Cash);
        assert_eq!(input.account_id, account_id("1120"));
        assert_eq!(input.id, None);
        let mut books = state.clone();
        let CommandResult::Id { id } = run(&mut books, LedgerCommand::BankAdd { input }) else {
            panic!("no id");
        };
        assert!(books.bank_accounts.iter().any(|b| b.id.as_str() == id));
    }

    const MPESA: &str = "Receipt No.,Completion Time,Details,Transaction Status,Paid In,Withdrawn\n\
        RB34NNA81C,2026-02-03 10:12,Funds received from Equity Bank,Completed,15000,\n\
        RXX1,2026-02-03 11:00,Pending transfer,Pending,,500\n\
        RZZ9,2026-02-05 09:30,Airtime,Completed,,250.50\n\
        RZZ9,2026-02-05 09:30,Airtime,Completed,,250.50\n\
        RYY2,,No date,Completed,100,\n";

    #[test]
    fn import_preview_maps_parse_statement_and_counts_repeats() {
        let state = demo();
        assert_eq!(ImportPreview::of(&state, &bank("mpesa-till"), "  \n"), None);
        let p = ImportPreview::of(&state, &bank("mpesa-till"), MPESA).unwrap();
        assert_eq!(p.format, StatementFormat::Mpesa);
        assert_eq!(p.rows.len(), 3);
        assert_eq!(
            p.skipped,
            ["Line 3: status is pending", "Line 6: no readable date"]
        );
        assert_eq!(p.money_in, 1_500_000);
        assert_eq!(p.money_out, -50_100);
        // The first row is already on the demo till statement (demo-005q).
        assert_eq!(p.repeats, 1);
        assert_eq!(p.to_add(), 2);
        assert_eq!(p.button_label(), "Import 3 lines");
        let command = p.command(&bank("mpesa-till")).unwrap();
        let mut books = state.clone();
        let CommandResult::Ids { ids } = run(&mut books, command) else {
            panic!("no ids");
        };
        assert_eq!(
            ids.len(),
            p.to_add(),
            "the preview's repeat count is the engine's"
        );
        assert_eq!(
            import_notice(ids.len(), p.rows.len()),
            "2 of 3 lines imported; 1 were already on this statement and were left out."
        );
        assert_eq!(
            import_notice(1, 1),
            "1 statement line imported. Run Auto-match to pair them with the books."
        );
        // Importing again repeats every row.
        let again = ImportPreview::of(&books, &bank("mpesa-till"), MPESA).unwrap();
        assert_eq!((again.repeats, again.to_add()), (3, 0));

        let empty = ImportPreview::of(&state, &bank("mpesa-till"), "Date,Amount\n,\n").unwrap();
        assert!(empty.rows.is_empty());
        assert_eq!(empty.button_label(), "Import");
        assert_eq!(empty.command(&bank("mpesa-till")), None);
    }

    #[test]
    fn statuses_on_the_demo() {
        let state = demo();
        let rows = line_rows(&state, &bank("bank-main"));
        let ids: Vec<_> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "demo-0057",
                "demo-0058",
                "demo-0059",
                "demo-0056",
                "demo-0055",
                "demo-0054",
                "demo-0053",
                "demo-0052"
            ],
            "latest first, ties in import order"
        );
        let status = |id: &str| rows.iter().find(|r| r.id.as_str() == id).unwrap().status;
        assert_eq!(status("demo-0052"), LineStatus::Auto);
        assert_eq!(
            status("demo-0058"),
            LineStatus::Posted,
            "JE-0022 was posted from the line"
        );
        assert_eq!(status("demo-0059"), LineStatus::Unmatched);
        let r52 = rows.iter().find(|r| r.id.as_str() == "demo-0052").unwrap();
        assert_eq!(r52.entry.as_ref().unwrap().1, "JE-0002");
        assert_eq!(count(&rows, None), 8);
        assert_eq!(count(&rows, Some(LineStatus::Auto)), 6);
        assert_eq!(count(&rows, Some(LineStatus::Posted)), 1);
        assert_eq!(count(&rows, Some(LineStatus::Manual)), 0);

        // A manual match of an existing entry is By hand, not Posted.
        let mut books = state.clone();
        run(&mut books, unmatch_command(&line("demo-0053")));
        let after = line_rows(&books, &bank("bank-main"));
        let s53 = after.iter().find(|r| r.id.as_str() == "demo-0053").unwrap();
        assert_eq!(s53.status, LineStatus::Unmatched);
        assert_eq!(s53.entry, None);
        let c = candidates(&books, &line("demo-0053"), MANUAL_WINDOW);
        assert_eq!(c.len(), 1);
        run(
            &mut books,
            match_command(&line("demo-0053"), &c[0].journal_line),
        );
        let s53 = line_rows(&books, &bank("bank-main"))
            .into_iter()
            .find(|r| r.id.as_str() == "demo-0053")
            .unwrap();
        assert_eq!(s53.status, LineStatus::Manual);
    }

    #[test]
    fn ambiguous_when_more_than_one_candidate() {
        let mut books = demo();
        // A second KES 12,300 payment on the till's date of demo-005o: import it, then post
        // it as a new entry and unmatch it, so two journal lines fit each of the two lines.
        let csv = "Date,Description,Amount\n2026-01-23,Payment A,-12300\n";
        let p = ImportPreview::of(&books, &bank("mpesa-till"), csv).unwrap();
        run(&mut books, p.command(&bank("mpesa-till")).unwrap());
        // Both statement lines (demo-005o and the new one) have one candidate, JE-0021.
        let rows = line_rows(&books, &bank("mpesa-till"));
        assert!(rows.iter().all(|r| r.status != LineStatus::Ambiguous));
        let new_line = rows
            .iter()
            .find(|r| r.description == "Payment A")
            .unwrap()
            .id
            .clone();
        let mut form = PostForm::new(
            &books,
            books
                .statement_lines
                .iter()
                .find(|s| s.id == new_line)
                .unwrap(),
        );
        form.account = Some(account_id("5100"));
        run(&mut books, form.command(&new_line).unwrap());
        run(&mut books, unmatch_command(&new_line));
        let rows = line_rows(&books, &bank("mpesa-till"));
        let s = rows.iter().find(|r| r.id.as_str() == "demo-005o").unwrap();
        assert_eq!(s.status, LineStatus::Ambiguous);
        assert_eq!(count(&rows, Some(LineStatus::Ambiguous)), 2);
    }

    #[test]
    fn candidate_order_follows_the_engine() {
        let mut books = demo();
        // Free every bank-main line, then each open line's candidates come back reference
        // matches first, then by day gap.
        for s in books.statement_lines.clone() {
            if s.bank_account_id.as_str() == "bank-main" && s.matched_line_id.is_some() {
                run(&mut books, unmatch_command(&s.id));
            }
        }
        for s in books
            .statement_lines
            .iter()
            .filter(|s| s.bank_account_id.as_str() == "bank-main")
        {
            let c = candidates(&books, &s.id, 30);
            let engine = ledger_core::reconcile::candidates_for(&books, s, 30, &[]);
            assert_eq!(
                c.iter().map(|x| x.journal_line.clone()).collect::<Vec<_>>(),
                engine.iter().map(|x| x.line.id.clone()).collect::<Vec<_>>()
            );
            for w in c.windows(2) {
                assert!(
                    (w[0].reference_match && !w[1].reference_match)
                        || (w[0].reference_match == w[1].reference_match
                            && w[0].day_gap <= w[1].day_gap),
                    "{w:?}"
                );
            }
        }
        let c = candidates(&books, &line("demo-0052"), AUTO_WINDOW);
        assert_eq!(c[0].number, "JE-0002");
        assert!(c[0].reference_match);
        assert_eq!(c[0].gap_text(), "Same day \u{b7} Reference matches");
        let far = CandidateRow {
            day_gap: 1,
            reference_match: false,
            ..c[0].clone()
        };
        assert_eq!(far.gap_text(), "1 day apart");
        let farther = CandidateRow { day_gap: 3, ..far };
        assert_eq!(farther.gap_text(), "3 days apart");
        assert!(candidates(&books, &line("nope"), 3).is_empty());
    }

    #[test]
    fn position_of_the_demo_accounts() {
        let state = demo();
        let p = position(&state, &bank("bank-main"), &as_of());
        assert_eq!(p.book_balance, 891_875_000);
        assert_eq!(
            p.span,
            Some((
                IsoDate::parse("2026-01-06").unwrap(),
                IsoDate::parse("2026-02-01").unwrap()
            ))
        );
        assert_eq!((p.matched, p.only_statement), (711_875_000, 341_200));
        assert_eq!((p.only_books, p.only_books_count), (-120_000_000, 1));
        assert_eq!(p.statement_net(), 712_216_200);
        assert_eq!(p.books_net(), 591_875_000);
        assert_eq!(p.difference(), 120_341_200);
        assert!(p.verdict().starts_with("The difference is"));
        let till = position(&state, &bank("mpesa-till"), &as_of());
        assert_eq!(
            (till.only_statement, till.only_books, till.only_books_count),
            (229_000, 270_000, 2)
        );
        let ops = position(&state, &bank("bank-ops"), &as_of());
        assert_eq!(ops.span, None);
        assert_eq!(ops.difference(), 0);
        assert_eq!(
            ops.verdict(),
            "The statement and the books agree over these dates."
        );
    }

    #[test]
    fn commands_and_auto_match_summary() {
        assert_eq!(
            auto_match_command(&bank("b")),
            LedgerCommand::StatementAutoMatch {
                bank_account_id: bank("b")
            }
        );
        assert_eq!(
            match_command(&line("s"), &LineId::from("j")),
            LedgerCommand::StatementMatch {
                statement_line_id: line("s"),
                journal_line_id: LineId::from("j")
            }
        );
        assert_eq!(
            unmatch_command(&line("s")),
            LedgerCommand::StatementUnmatch {
                statement_line_id: line("s")
            }
        );
        let mut books = demo();
        let CommandResult::AutoMatch { result } =
            run(&mut books, auto_match_command(&bank("mpesa-till")))
        else {
            panic!("no result");
        };
        assert_eq!(
            auto_match_summary(&result),
            [
                "2 lines matched".to_owned(),
                "0 lines need a decision".to_owned(),
                "1 line unmatched".to_owned()
            ]
        );
    }

    #[test]
    fn post_form_builds_statement_post() {
        let state = demo();
        let s = state
            .statement_lines
            .iter()
            .find(|s| s.id.as_str() == "demo-0059")
            .unwrap();
        let mut form = PostForm::new(&state, s);
        assert_eq!(
            form.fund.as_ref().map(FundId::as_str),
            Some(UNRESTRICTED_FUND_ID)
        );
        assert_eq!(form.memo, "INTEREST CREDIT");
        assert_eq!(form.account, None);
        form.grant = Some(GrantId::from("g"));
        form.set_fund(None);
        assert_eq!(form.grant, None, "a new fund clears the grant");
        assert_eq!(
            form.command(&s.id).unwrap_err(),
            "Choose the fund this money belongs to."
        );
        form.set_fund(Some(FundId::from(UNRESTRICTED_FUND_ID)));
        form.memo = "  ".to_owned();
        let LedgerCommand::StatementPost {
            statement_line_id,
            counter,
        } = form.command(&s.id).unwrap()
        else {
            panic!("not statement.post");
        };
        assert_eq!(statement_line_id, s.id);
        assert_eq!(counter.account_id, None, "empty account posts to suspense");
        assert_eq!(counter.grant_id, Nullish::Null);
        assert_eq!(counter.memo, None);
        assert!(
            suspense_hint(&state).starts_with("Leave empty to hold it in 1400 "),
            "{}",
            suspense_hint(&state)
        );

        // Money in offers income, asset and liability accounts; money out expense, asset and
        // liability, by type then code.
        let types = |amount| {
            post_accounts(&state, amount)
                .iter()
                .map(|(id, _)| state.account(id).unwrap().account_type)
                .collect::<Vec<_>>()
        };
        let inflow = types(1);
        assert!(inflow.iter().all(|t| matches!(
            t,
            ledger_core::types::AccountType::Asset
                | ledger_core::types::AccountType::Liability
                | ledger_core::types::AccountType::Income
        )));
        assert!(inflow.contains(&ledger_core::types::AccountType::Income));
        let outflow = types(-1);
        assert!(outflow.contains(&ledger_core::types::AccountType::Expense));
        assert!(!outflow.contains(&ledger_core::types::AccountType::Income));
        assert!(outflow.windows(2).all(|w| w[0] as u8 <= w[1] as u8));

        // Posting interest income marks the line Posted.
        let mut books = state.clone();
        form.account = Some(account_id("4300"));
        run(&mut books, form.command(&s.id).unwrap());
        let rows = line_rows(&books, &bank("bank-main"));
        assert_eq!(
            rows.iter().find(|r| r.id == s.id).unwrap().status,
            LineStatus::Posted
        );
    }
}
