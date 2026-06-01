// "/" — spins up a fresh chat and hands off to /c/:id (which renders the empty
// state + prompt suggestions). Mirrors open-webui landing on a new conversation.

import { useEffect, useRef } from "react";
import { useNavigate } from "@tanstack/react-router";
import { MdCircularProgress } from "@aphrody-code/m3-react";
import { useCreateChat } from "../api/queries.ts";

export function Home() {
  const createChat = useCreateChat();
  const navigate = useNavigate();
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void createChat
      .mutateAsync({ title: "New Chat" })
      .then((chat) => navigate({ to: "/c/$chatId", params: { chatId: chat.id }, replace: true }));
  }, []);

  return (
    <div className="owui-empty">
      <MdCircularProgress indeterminate />
    </div>
  );
}
