# When to Mock

Mock at **system boundaries** only, and only IF no real or local stand-in exists (test DB, temp dir, sandbox, clock that test passes in). In Rust: stand-in = coarse test adapter behind trait, in contract or integration test. Never mock in unit test (`rex-code-tests-unit-testing`).

System boundaries:

- External APIs (payment, email, etc.)
- Databases (sometimes - prefer test DB)
- Time/randomness
- File system (sometimes)

Do not mock:

- Your own classes/modules
- Internal collaborators
- Anything you control

## Design for Mockability

At system boundaries, design interfaces that are easy to mock:

**1. Use dependency injection**

Give external dependencies as parameters. Do not create them internally:

```typescript
// Easy to mock
function processPayment(order, paymentClient) {
  return paymentClient.charge(order.total);
}

// Hard to mock
function processPayment(order) {
  const client = new StripeClient(process.env.STRIPE_KEY);
  return client.charge(order.total);
}
```

**2. Prefer SDK-style interfaces over generic fetchers**

Create specific functions for each external operation instead of one generic function with conditional logic:

```typescript
// GOOD: Each function is independently mockable
const api = {
  getUser: (id) => fetch(`/users/${id}`),
  getOrders: (userId) => fetch(`/users/${userId}/orders`),
  createOrder: (data) => fetch('/orders', { method: 'POST', body: data }),
};

// BAD: Mocking requires conditional logic inside the mock
const api = {
  fetch: (endpoint, options) => fetch(endpoint, options),
};
```

SDK approach gives:
- Each mock returns one specific shape
- No conditional logic in test setup
- Easier to see which endpoints test exercises
- Type safety per endpoint
