import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { Button } from "@1io/kui-button";
import { TextInput } from "@1io/kui-text-input";
import { Form, FormItem } from "@1io/kui-form";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    try {
      // test for core state commands
      let a = await invoke("get_core_state", { co: "local", core: "membership" });
      console.log("TESTT", a);
      let tmp: string = await invoke("tmp_test_command", { name });
      setGreetMsg(tmp);
    } catch (e) {
      console.log(e);
    }
  }

  return (
    <div className="container">
      <h1>Welcome to Tauri!</h1>

      <div className="row">
        <a href="https://vitejs.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://reactjs.org" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>

      <p>Click on the Tauri, Vite, and React logos to learn more.</p>
      <Form>
        <FormItem label="">
          <TextInput value={name} onChange={setName} onCommit={setName} />
        </FormItem>

        <FormItem label="">
          <Button label="Greet" onClick={greet} />
        </FormItem>
      </Form>

      <p>{greetMsg}</p>
    </div>
  );
}

export default App;
