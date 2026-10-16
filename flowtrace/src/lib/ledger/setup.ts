import { UNRESTRICTED_FUND_ID, defaultSettings, ngoChartOfAccounts } from './coa-template'
import { Ledger, defaultEnv, emptyLedger } from './engine'
import type { LedgerEnv } from './engine'
import type { Actor } from './types'

/**
 * A new organisation's books: the NGO chart of accounts, an unrestricted
 * fund and the twelve periods of the current fiscal year. Accrual basis,
 * IFRS for SMEs presentation, one base currency (the plan's defaults).
 */
export function createOrgLedger(
  organizationId: string,
  baseCurrency: string,
  fiscalYear: number,
  env: LedgerEnv = defaultEnv,
) {
  const state = emptyLedger(defaultSettings(organizationId, baseCurrency))
  state.accounts = ngoChartOfAccounts()
  state.funds = [
    { id: UNRESTRICTED_FUND_ID, name: 'Unrestricted', restriction: 'unrestricted' },
  ]
  const ledger = new Ledger(state, env)
  ledger.ensureFiscalYear(fiscalYear)
  return ledger
}

/** The engine's view of a member, from their access-group permissions. */
export function actorFromPermissions(
  user: { id: string; name: string },
  perms: {
    can_edit_transactions: boolean
    can_manage_groups: boolean
    can_manage_members?: boolean
  },
): Actor {
  return {
    id: user.id,
    name: user.name,
    can_post: perms.can_edit_transactions,
    can_approve: perms.can_edit_transactions,
    can_close: perms.can_manage_groups,
  }
}

export const SYSTEM_ACTOR: Actor = {
  id: 'system',
  name: 'FlowTrace',
  can_post: true,
  can_approve: true,
  can_close: true,
}
