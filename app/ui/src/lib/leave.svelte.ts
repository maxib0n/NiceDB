export type LeaveChoice = 'apply' | 'discard' | false;

export const leave = $state({ ask: null as { count: number; resolve: (choice: LeaveChoice) => void } | null });

export function askLeave(count: number): Promise<LeaveChoice> {
  leave.ask?.resolve(false);
  return new Promise((resolve) => {
    leave.ask = {
      count,
      resolve: (choice) => {
        leave.ask = null;
        resolve(choice);
      },
    };
  });
}
