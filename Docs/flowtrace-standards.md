# FlowTrace Development Standards

## Code Quality

### TypeScript Configuration

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "paths": { "@/*": ["./src/*"] }
  }
}
```

### ESLint

Using `@tanstack/eslint-config`:

```bash
npm run lint      # Check
npm run lint:fix  # Auto-fix
```

### Prettier

```json
{
  "semi": false,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "es5"
}
```

```bash
npm run format    # Format all files
npm run check     # Format + lint
```

---

## Component Patterns

### Structure

```typescript
// 1. Imports (external, internal, types)
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import type { Transaction } from '@/types'

// 2. Types
interface Props {
  transaction: Transaction
  onVerify?: (id: string) => void
}

// 3. Component
export function TransactionCard({ transaction, onVerify }: Props) {
  // Hooks first
  const [expanded, setExpanded] = useState(false)

  // Handlers
  const handleVerify = () => onVerify?.(transaction.id)

  // Early returns
  if (!transaction) return null

  // Render
  return (/* JSX */)
}
```

### Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Components | PascalCase | `TransactionCard.tsx` |
| Hooks | camelCase with `use` | `useTransactions.ts` |
| Utilities | camelCase | `formatCurrency.ts` |
| Types | PascalCase | `Transaction`, `UserRole` |
| Constants | SCREAMING_SNAKE | `MAX_FILE_SIZE` |
| Boolean props | `is`, `has`, `should` | `isLoading`, `hasError` |
| Event handlers | `on` + verb | `onClick`, `onSubmit` |

---

## Git Workflow

### Branch Naming

```
feature/ft-001-description
bugfix/bf-001-description
hotfix/hf-001-critical-issue
release/v1.0.0
```

### Commit Convention

```
<type>(<scope>): <subject>

Types: feat, fix, docs, style, refactor, perf, test, chore
```

**Examples:**
```bash
git commit -m "feat(auth): add WorkOS SSO integration"
git commit -m "fix(transactions): correct amount parsing"
git commit -m "docs(api): update endpoint documentation"
```

### Pull Request Process

1. Create feature branch from `develop`
2. Make changes with conventional commits
3. Run `npm run check` before pushing
4. Open PR with description template
5. Request review, address feedback
6. Squash merge to `develop`

### PR Checklist

- [ ] Code follows style guide
- [ ] Tests pass (`npm run test`)
- [ ] No TypeScript errors
- [ ] Mobile responsive
- [ ] Accessibility checked

---

## Testing

### Stack

- **Unit/Integration**: Vitest + Testing Library
- **E2E**: Playwright
- **API Mocking**: MSW (Mock Service Worker)

### Running Tests

```bash
npm run test           # Run all tests
npm run test:watch     # Watch mode
npm run test:coverage  # With coverage
npm run test:e2e       # Playwright E2E
```

### Test File Structure

```
src/
├── components/
│   └── TransactionCard/
│       ├── TransactionCard.tsx
│       └── TransactionCard.test.tsx
├── hooks/
│   └── useTransactions.test.ts
└── lib/
    └── utils.test.ts
```

### Component Test Example

```typescript
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TransactionCard } from './TransactionCard'

describe('TransactionCard', () => {
  const mockTransaction = {
    id: 'txn_123',
    amount: 1000,
    description: 'Test',
    verificationStatus: 'pending',
  }

  it('renders transaction details', () => {
    render(<TransactionCard transaction={mockTransaction} />)
    expect(screen.getByText('Test')).toBeInTheDocument()
  })

  it('calls onVerify when clicked', async () => {
    const onVerify = vi.fn()
    render(<TransactionCard transaction={mockTransaction} onVerify={onVerify} />)

    await userEvent.click(screen.getByRole('button', { name: /verify/i }))
    expect(onVerify).toHaveBeenCalledWith('txn_123')
  })
})
```

### Hook Test Example

```typescript
import { renderHook, waitFor } from '@testing-library/react'
import { useTransactions } from './useTransactions'

describe('useTransactions', () => {
  it('fetches transactions', async () => {
    const { result } = renderHook(() => useTransactions('org_123'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.transactions).toHaveLength(3)
  })
})
```

### Coverage Targets

| Metric | Target |
|--------|--------|
| Statements | 80% |
| Branches | 75% |
| Functions | 80% |
| Lines | 80% |

---

## Accessibility

### Requirements

- WCAG 2.1 AA compliance
- Keyboard navigation for all interactions
- Screen reader friendly with proper ARIA labels
- Color contrast ratio 4.5:1 minimum

### Patterns

```tsx
// Button with ARIA label
<button
  aria-label="Verify transaction"
  aria-pressed={isVerified}
  onClick={handleVerify}
>
  <CheckIcon aria-hidden="true" />
</button>

// Loading state announcement
<div role="status" aria-live="polite">
  {isLoading && <span className="sr-only">Loading transactions...</span>}
</div>
```

---

## Performance

### Bundle Size

Monitor with `npm run build` output. Target < 200KB initial JS.

### Code Splitting

```typescript
const FlowVisualization = lazy(() => import('@/components/FlowVisualization'))
```

### Image Optimization

- Use WebP format
- Lazy load below-fold images
- Responsive sizes with srcset
- Max 100KB per image

### Data Fetching

```typescript
useQuery({
  queryKey: ['transactions', orgId],
  queryFn: fetchTransactions,
  staleTime: 5 * 60 * 1000,  // 5 min cache
  refetchOnWindowFocus: false,
})
```

---

## Security

### Input Validation

Use Zod for runtime validation:

```typescript
import { z } from 'zod'

const TransactionSchema = z.object({
  amount: z.number().positive(),
  description: z.string().min(1).max(500),
  currency: z.enum(['USD', 'KES', 'NGN', 'GHS']),
})
```

### API Security

- All endpoints require authentication (except `/public/*`)
- Rate limiting enforced
- Input sanitization for XSS prevention
- CORS configured for allowed origins

### Environment Variables

Never commit `.env` files. Use:
- `.env.example` for templates
- GitHub Secrets for CI/CD
- Vercel/hosting environment variables

---

## Documentation

### Component JSDoc

```typescript
/**
 * Displays transaction with verification status
 *
 * @example
 * <TransactionCard
 *   transaction={txn}
 *   onVerify={handleVerify}
 * />
 */
export function TransactionCard({ ... }) { }
```

### API Documentation

```typescript
/**
 * Create a new transaction
 *
 * @endpoint POST /api/transactions
 * @body { amount: number, fromNodeId: string, toNodeId: string }
 * @returns Transaction
 */
```

---

## CI/CD

### GitHub Actions

```yaml
# .github/workflows/ci.yml
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm ci
      - run: npm run check
      - run: npm run test
      - run: npm run build
```

### Pre-commit Hooks

```json
// package.json
{
  "scripts": {
    "prepare": "husky"
  }
}
```

```bash
# .husky/pre-commit
npm run check
npm run test --run
```

---

## Quick Reference

### Commands

```bash
npm run dev        # Start dev server
npm run build      # Production build
npm run preview    # Preview build
npm run test       # Run tests
npm run check      # Format + lint
```

### Import Aliases

```typescript
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useAuth } from '@/hooks/useAuth'
import type { Transaction } from '@/types'
```

### Adding Components

```bash
npx shadcn add button card input dialog
```
