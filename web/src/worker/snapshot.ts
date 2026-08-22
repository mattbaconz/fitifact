export class StaleConstraints extends Error {}

export function withMatchingConstraints<T>(
  storedSnapshot: string,
  expectedSnapshot: string,
  operation: () => T,
): T {
  if (storedSnapshot !== expectedSnapshot) {
    throw new StaleConstraints("The target changed after this source or plan was reviewed.");
  }
  return operation();
}
