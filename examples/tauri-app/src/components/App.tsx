import "./App.css";
import * as React from "react";
import { LevelStack } from "@1io/kui-level-stack";
import { MessengerView } from "@1io/coapp-messenger-view";
import { MessengerActionType, MessengerSendAction } from "../actions";
import { useDispatch, useSelector } from "react-redux";
import { MessengerPluginState } from "../state";

interface AppProps { }

export function App(props: AppProps) {

  const dispatch = useDispatch();
  const [message, setMessage] = React.useState("");
  const messages = useSelector((state: MessengerPluginState) => state.messages);

  const onSendMessage = () => {
    dispatch<MessengerSendAction>({
      type: MessengerActionType.Send,
      payload: { message }
    });
    setMessage("");
  }

  return <LevelStack style={{ width: "100%", height: "100%" }}>
    <MessengerView
      chatInput={message}
      chatName={"Some test chat"}
      onChatInput={setMessage}
      messages={messages}
      onSendMessage={onSendMessage}
    />
  </ LevelStack>;

}

