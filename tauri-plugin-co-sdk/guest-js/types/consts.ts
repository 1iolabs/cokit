export const TODO_CORE_NAME: string = "todo";
export const TODO_IDENTITY_NAME: string = "my-todo-identity";
export const CO_CORE_NAME_MEMBERSHIP = "membership";

export async function fetchTodoCoreBinary(): Promise<ReadableStream<Uint8Array>> {
  const response = await fetch("my_todo_core.wasm");
  const stream = response.body;
  if (stream === null) {
    throw new Error("Todo core binary stream is null");
  }
  return stream;
}
