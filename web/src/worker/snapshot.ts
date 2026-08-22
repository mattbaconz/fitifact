export class StaleConstraints extends Error {}

export function withMatchingConstraints<T>(
  storedSnapshot: string | null,
  expectedSnapshot: string,
  operation: () => T,
): T {
  if (storedSnapshot === null) return operation();
  if (storedSnapshot !== expectedSnapshot) {
    throw new StaleConstraints("The target changed after this source or plan was reviewed.");
  }
  return operation();
}
