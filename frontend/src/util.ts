export function assert(expression: boolean, message: string = 'unknown error') {
  if (!expression) {
    debugger;
    throw Error(`assertion failed: ${message}`);
  }
}
