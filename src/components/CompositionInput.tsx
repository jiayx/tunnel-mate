import { useEffect, useRef, useState, type ChangeEvent, type InputHTMLAttributes, type KeyboardEvent, type TextareaHTMLAttributes } from "react";

type CompositionInputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "defaultValue" | "onChange"> & {
  value: string;
  onValueChange: (value: string) => void;
};

type CompositionTextareaProps = Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, "value" | "defaultValue" | "onChange"> & {
  value: string;
  onValueChange: (value: string) => void;
};

const pinyinCharsPattern = /^[A-Za-züÜvV\s]+$/;

function normalizeCommittedPinyin(baseValue: string, nextValue: string) {
  if (baseValue === nextValue) {
    return nextValue;
  }

  let prefixLength = 0;
  while (
    prefixLength < baseValue.length &&
    prefixLength < nextValue.length &&
    baseValue[prefixLength] === nextValue[prefixLength]
  ) {
    prefixLength += 1;
  }

  let suffixLength = 0;
  while (
    suffixLength < baseValue.length - prefixLength &&
    suffixLength < nextValue.length - prefixLength &&
    baseValue[baseValue.length - 1 - suffixLength] === nextValue[nextValue.length - 1 - suffixLength]
  ) {
    suffixLength += 1;
  }

  const insertedEnd = suffixLength === 0 ? nextValue.length : nextValue.length - suffixLength;
  const inserted = nextValue.slice(prefixLength, insertedEnd);
  if (!inserted.includes(" ") || !pinyinCharsPattern.test(inserted)) {
    return nextValue;
  }

  return `${nextValue.slice(0, prefixLength)}${inserted.replace(/\s+/g, "")}${nextValue.slice(insertedEnd)}`;
}

export function CompositionInput({ value, onValueChange, onCompositionStart, onCompositionEnd, onBlur, onKeyDown, ...props }: CompositionInputProps) {
  const [draft, setDraft] = useState(value);
  const composingRef = useRef(false);
  const compositionBaseRef = useRef(value);
  const tabCommitRef = useRef(false);

  useEffect(() => {
    if (!composingRef.current) {
      setDraft(value);
    }
  }, [value]);

  const commit = (nextValue: string, normalizePinyin = false) => {
    const committedValue = normalizePinyin || tabCommitRef.current
      ? normalizeCommittedPinyin(compositionBaseRef.current, nextValue)
      : nextValue;
    tabCommitRef.current = false;
    setDraft(committedValue);
    onValueChange(committedValue);
  };

  return (
    <input
      {...props}
      value={draft}
      onChange={(event: ChangeEvent<HTMLInputElement>) => {
        const nextValue = event.target.value;
        setDraft(nextValue);
        if (!composingRef.current) {
          onValueChange(nextValue);
        }
      }}
      onCompositionStart={(event) => {
        composingRef.current = true;
        compositionBaseRef.current = value;
        tabCommitRef.current = false;
        onCompositionStart?.(event);
      }}
      onCompositionEnd={(event) => {
        composingRef.current = false;
        commit(event.currentTarget.value, true);
        onCompositionEnd?.(event);
      }}
      onBlur={(event) => {
        if (draft !== value) {
          commit(event.currentTarget.value);
        }
        onBlur?.(event);
      }}
      onKeyDown={(event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key === "Tab" && (composingRef.current || event.nativeEvent.isComposing)) {
          tabCommitRef.current = true;
        }
        onKeyDown?.(event);
      }}
    />
  );
}

export function CompositionTextarea({ value, onValueChange, onCompositionStart, onCompositionEnd, onBlur, onKeyDown, ...props }: CompositionTextareaProps) {
  const [draft, setDraft] = useState(value);
  const composingRef = useRef(false);
  const compositionBaseRef = useRef(value);
  const tabCommitRef = useRef(false);

  useEffect(() => {
    if (!composingRef.current) {
      setDraft(value);
    }
  }, [value]);

  const commit = (nextValue: string, normalizePinyin = false) => {
    const committedValue = normalizePinyin || tabCommitRef.current
      ? normalizeCommittedPinyin(compositionBaseRef.current, nextValue)
      : nextValue;
    tabCommitRef.current = false;
    setDraft(committedValue);
    onValueChange(committedValue);
  };

  return (
    <textarea
      {...props}
      value={draft}
      onChange={(event: ChangeEvent<HTMLTextAreaElement>) => {
        const nextValue = event.target.value;
        setDraft(nextValue);
        if (!composingRef.current) {
          onValueChange(nextValue);
        }
      }}
      onCompositionStart={(event) => {
        composingRef.current = true;
        compositionBaseRef.current = value;
        tabCommitRef.current = false;
        onCompositionStart?.(event);
      }}
      onCompositionEnd={(event) => {
        composingRef.current = false;
        commit(event.currentTarget.value, true);
        onCompositionEnd?.(event);
      }}
      onBlur={(event) => {
        if (draft !== value) {
          commit(event.currentTarget.value);
        }
        onBlur?.(event);
      }}
      onKeyDown={(event: KeyboardEvent<HTMLTextAreaElement>) => {
        if (event.key === "Tab" && (composingRef.current || event.nativeEvent.isComposing)) {
          tabCommitRef.current = true;
        }
        onKeyDown?.(event);
      }}
    />
  );
}
