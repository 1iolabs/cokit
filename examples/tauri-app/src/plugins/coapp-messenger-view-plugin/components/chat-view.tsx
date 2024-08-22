import * as React from "react";
import { LevelStack } from "@1io/kui-level-stack";
import { MessengerView } from "@1io/coapp-messenger-view";
import { MessengerViewActionType, MessengerViewSendAction } from "../actions";
import { useDispatch, useSelector } from "react-redux";
import { MessengerViewPluginState } from "../state";

interface MessengerViewProps { }

export function MessengerViewContainer(props: MessengerViewProps) {

  const dispatch = useDispatch();
  const [message, setMessage] = React.useState("");
  const messages = useSelector((state: MessengerViewPluginState) => state.messages);
  const chatName = useSelector((state: MessengerViewPluginState) => state.chatName);

  const onSendMessage = () => {
    dispatch<MessengerViewSendAction>({
      type: MessengerViewActionType.Send,
      payload: { message }
    });
    setMessage("");
  }

  return <LevelStack style={{ width: "100%", height: "100%" }}>
    <MessengerView
      chatInput={message}
      chatName={chatName}
      onChatInput={setMessage}
      messages={messages}
      onSendMessage={onSendMessage}
    />
  </ LevelStack>;

}

