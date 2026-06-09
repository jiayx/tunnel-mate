import { useState } from "react";

type NoticeType = "error" | "success";

export interface NoticeState {
  message: string;
  type: NoticeType;
}

export interface ConfirmState {
  message: string;
  onConfirm: () => void;
}

export function useAppFeedback() {
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const [confirmRequest, setConfirmRequest] = useState<ConfirmState | null>(null);

  const showNotice = (message: string, type: NoticeType = "success") => {
    setNotice({ message, type });
    window.setTimeout(() => setNotice(null), 3500);
  };

  const requestConfirm = (message: string, onConfirm: () => void) => {
    setConfirmRequest({ message, onConfirm });
  };

  return {
    notice,
    confirmRequest,
    setConfirmRequest,
    showNotice,
    requestConfirm,
  };
}
