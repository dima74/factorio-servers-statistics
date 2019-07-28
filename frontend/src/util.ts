export function assert(expression: boolean, message: string = 'unknown error') {
  if (!expression) {
  debugger;
    throw Error(`assertion failed: ${message}`);
  }
}

export function timeMinutesToDate(timeMinutes: TimeMinutes): Date {
  return new Date(timeMinutes * 60 * 1000);
}

export function dateToTimeMinutes(date: Date): TimeMinutes {
  return date.getTime() / 1000 / 60;
}
