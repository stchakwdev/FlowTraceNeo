# supabase/ (historical)

The app no longer uses Supabase: every record is served by the SpacetimeDB module or the demo
dataset (RW1-RW3). These migrations stay as the documented source of the old row-level security
rules that the module's views and reducers mirror (rust/docs/adr/0004-transparency-tables.md and
0005 cite them by file and line). Do not apply them anywhere.
