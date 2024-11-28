import { MessengerView } from "@1io/coapp-messenger-view";
import { LevelStack } from "@1io/kui-level-stack";
import * as React from "react";
import { useDispatch, useSelector } from "react-redux";
import { identity } from "rxjs";
import { MessengerViewActionType, MessengerViewLoadMoreEventsAction, MessengerViewSendAction } from "../actions/index.js";
import { MessengerViewPluginState } from "../state/index.js";

export interface MessengerViewContainerProps {
  onBack: () => void;
}

export function MessengerViewContainer(props: MessengerViewContainerProps) {

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

  const onScrollTop = () => {
    dispatch(identity<MessengerViewLoadMoreEventsAction>({
      payload: { count: 30 },
      type: MessengerViewActionType.LoadMoreEvents,
    }));
  }

  return <LevelStack style={{ width: "100%", height: "100%" }}>
    <MessengerView
      chatInput={message}
      chatName={chatName}
      onChatInput={setMessage}
      messages={messages}
      onSendMessage={onSendMessage}
      onBack={props.onBack}
      onScrollTop={onScrollTop}
    />
  </ LevelStack>;

}

