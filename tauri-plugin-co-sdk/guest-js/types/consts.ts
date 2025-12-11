export const CO_CORE_NAME_MEMBERSHIP = "membership";

export async function fetchBinary(name: string): Promise<ReadableStream<Uint8Array>> {
  const response = await fetch(name);
  const stream = response.body;
  if (stream === null) {
    throw new Error("Core binary stream is null");
  }
  return stream;
}
