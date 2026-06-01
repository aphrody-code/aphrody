// "/c/:chatId" — loads the chat by id and renders the ChatView.

import { useParams } from "@tanstack/react-router";
import { MdCircularProgress } from "@aphrody-code/m3-react";
import { ChatView } from "../components/chat/ChatView.tsx";
import { useChat } from "../api/queries.ts";

export function ChatRoute() {
  const { chatId } = useParams({ strict: false }) as { chatId: string };
  const { data: chat, isLoading } = useChat(chatId);

  if (isLoading) {
    return (
      <div className="owui-empty">
        <MdCircularProgress indeterminate />
      </div>
    );
  }
  return <ChatView chat={chat} />;
}
