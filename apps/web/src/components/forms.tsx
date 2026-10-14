import { useId, useState, type ReactNode } from "react";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import {
  Field,
  FieldLabel,
  FieldDescription,
  FieldError,
} from "@/components/ui/field";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectGroup,
  SelectItem,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Alert, AlertTitle, AlertDescription } from "@/components/ui/alert";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
} from "@/components/ui/empty";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { ApiFailure } from "@/lib/api";

export function TextField({
  label,
  value,
  onChange,
  multiline = false,
  type = "text",
  required = false,
  maxLength = 4096,
  description,
  disabled = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  multiline?: boolean;
  type?: string;
  required?: boolean;
  maxLength?: number;
  description?: string;
  disabled?: boolean;
}) {
  const id = useId();
  const [validation, setValidation] = useState("");
  const describedBy =
    [description ? `${id}-hint` : "", validation ? `${id}-error` : ""]
      .filter(Boolean)
      .join(" ") || undefined;
  return (
    <Field data-disabled={disabled} data-invalid={Boolean(validation)}>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      {multiline ? (
        <Textarea
          id={id}
          value={value}
          onChange={(e) => {
            setValidation("");
            onChange(e.target.value);
          }}
          aria-invalid={Boolean(validation)}
          aria-describedby={describedBy}
          onInvalid={(e) => setValidation(e.currentTarget.validationMessage)}
          maxLength={maxLength}
          disabled={disabled}
          required={required}
        />
      ) : (
        <Input
          id={id}
          type={type}
          value={value}
          onChange={(e) => {
            setValidation("");
            onChange(e.target.value);
          }}
          aria-invalid={Boolean(validation)}
          aria-describedby={describedBy}
          onInvalid={(e) => setValidation(e.currentTarget.validationMessage)}
          minLength={type === "password" ? 12 : undefined}
          maxLength={maxLength}
          required={required}
          disabled={disabled}
          autoComplete={type === "password" ? "new-password" : undefined}
        />
      )}{" "}
      {description ? (
        <FieldDescription id={`${id}-hint`}>{description}</FieldDescription>
      ) : null}
      {validation ? (
        <FieldError id={`${id}-error`}>{validation}</FieldError>
      ) : null}
    </Field>
  );
}
export function Choice({
  label,
  value,
  onChange,
  choices,
  disabled = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  choices: readonly { value: string; label: string }[];
  disabled?: boolean;
}) {
  const id = useId();
  return (
    <Field data-disabled={disabled}>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Select value={value} onValueChange={onChange} disabled={disabled}>
        <SelectTrigger id={id} className="w-full">
          <SelectValue placeholder={label} />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {choices.map((c) => (
              <SelectItem key={c.value} value={c.value}>
                {c.label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </Field>
  );
}
export function Feedback({
  error,
  reload,
}: {
  error: unknown;
  reload?: () => void;
}) {
  if (!error) return null;
  const conflict = error instanceof ApiFailure && error.code === "conflict";
  return (
    <Alert variant="destructive">
      <AlertTitle>
        {conflict ? "Changes need review" : "Unable to complete the request"}
      </AlertTitle>
      <AlertDescription>
        {error instanceof Error
          ? error.message
          : "The request could not be completed."}
        {conflict ? (
          <p>
            The server changed. Reload current state, review your edit, and
            submit again.
          </p>
        ) : null}
        {reload ? (
          <Button type="button" variant="outline" onClick={reload}>
            {conflict ? "Reload current state" : "Retry"}
          </Button>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}
export function NoData({
  title,
  description,
}: {
  title: string;
  description?: string;
}) {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyTitle>{title}</EmptyTitle>
        {description ? (
          <EmptyDescription>{description}</EmptyDescription>
        ) : null}
      </EmptyHeader>
    </Empty>
  );
}
export function Editor({
  title,
  description,
  children,
  onClose,
  wide = false,
  hideDescription = false,
  busy = false,
}: {
  title: string;
  description: string;
  children: ReactNode;
  onClose: () => void;
  wide?: boolean;
  hideDescription?: boolean;
  busy?: boolean;
}) {
  const [returnFocus] = useState(() =>
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null,
  );
  return (
    <Sheet
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}
    >
      <SheetContent
        aria-busy={busy}
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          if (returnFocus?.isConnected) returnFocus.focus();
        }}
        className={wide ? "editor editor-wide" : "editor"}
        onEscapeKeyDown={(e) => {
          if (busy) e.preventDefault();
        }}
        onPointerDownOutside={(e) => {
          if (busy) e.preventDefault();
        }}
      >
        <SheetHeader>
          <SheetTitle className="min-w-0 truncate pr-8">{title}</SheetTitle>
          <SheetDescription className={hideDescription ? "sr-only" : undefined}>
            {description}
          </SheetDescription>
        </SheetHeader>
        {busy ? (
          <p className="sr-only" role="status">
            <Spinner />
            Saving
          </p>
        ) : null}
        {children}
      </SheetContent>
    </Sheet>
  );
}
