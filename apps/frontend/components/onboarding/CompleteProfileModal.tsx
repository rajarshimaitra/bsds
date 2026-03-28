"use client";

/**
 * CompleteProfileModal
 *
 * Shown to newly onboarded staff/members who haven't yet filled in their
 * membership profile. Collects name, phone, address, and optional sub-members.
 *
 * After successful submission, the parent should call onSuccess() which
 * typically triggers a data revalidation.
 *
 * Props:
 *   open       — controlled open state
 *   onSuccess  — called after profile is saved (parent should mutate/revalidate)
 *   onSkip     — called when user clicks "Skip for now"
 *   prefillName — optional name pre-fill from auth user record
 */

import { useState, FormEvent } from "react";
import { apiFetch } from "@/lib/api-client";
import { useAuth } from "@/hooks/use-auth";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { PlusIcon, Trash2Icon } from "lucide-react";

interface SubMemberInput {
  name: string;
  email: string;
  phone: string;
  relation: string;
}

const EMPTY_SUB: SubMemberInput = { name: "", email: "", phone: "", relation: "" };

interface Props {
  open: boolean;
  onSuccess: () => void;
  onSkip: () => void;
  prefillName?: string;
}

export function CompleteProfileModal({ open, onSuccess, onSkip, prefillName }: Props) {
  const { refresh } = useAuth();

  const [name, setName] = useState(prefillName ?? "");
  const [phone, setPhone] = useState("");
  const [address, setAddress] = useState("");
  const [subMembers, setSubMembers] = useState<SubMemberInput[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  function addSubMember() {
    if (subMembers.length >= 3) return;
    setSubMembers((prev) => [...prev, { ...EMPTY_SUB }]);
  }

  function removeSubMember(idx: number) {
    setSubMembers((prev) => prev.filter((_, i) => i !== idx));
  }

  function updateSubMember(idx: number, field: keyof SubMemberInput, value: string) {
    setSubMembers((prev) =>
      prev.map((sm, i) => (i === idx ? { ...sm, [field]: value } : sm))
    );
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);

    if (!name.trim() || !phone.trim() || !address.trim()) {
      setError("Name, phone, and address are required.");
      return;
    }

    // Validate sub-members — if any row has a name it must also have email
    for (const sm of subMembers) {
      if (sm.name.trim() && !sm.email.trim()) {
        setError("Each sub-member with a name must also have an email.");
        return;
      }
    }

    setSubmitting(true);
    try {
      const res = await apiFetch("/api/onboarding/profile", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: name.trim(),
          phone: phone.trim(),
          address: address.trim(),
          subMembers: subMembers.filter((sm) => sm.name.trim() && sm.email.trim()),
        }),
      });

      const json = await res.json();

      if (!res.ok) {
        setError(json.error ?? "Failed to save profile. Please try again.");
        return;
      }

      // Refresh auth context so member_id in session updates
      await refresh();
      onSuccess();
    } catch {
      setError("An unexpected error occurred. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) onSkip(); }}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="text-lg">Complete your membership profile</DialogTitle>
          <DialogDescription>
            Fill in your personal details to activate your membership record.
            You can add up to 3 family sub-members. You can skip this and come back later.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-5 py-2">
          {error && (
            <div className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
              {error}
            </div>
          )}

          {/* Personal details */}
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="profile-name">Full name</Label>
              <Input
                id="profile-name"
                placeholder="Your full name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                disabled={submitting}
                required
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="profile-phone">WhatsApp / Phone</Label>
              <Input
                id="profile-phone"
                placeholder="10-digit mobile number"
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                disabled={submitting}
                required
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="profile-address">Residential address</Label>
              <Input
                id="profile-address"
                placeholder="Your current address"
                value={address}
                onChange={(e) => setAddress(e.target.value)}
                disabled={submitting}
                required
              />
            </div>
          </div>

          {/* Sub-members */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium text-slate-700">
                Sub-members{" "}
                <span className="font-normal text-muted-foreground">
                  (family, up to 3)
                </span>
              </p>
              {subMembers.length < 3 && (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={addSubMember}
                  disabled={submitting}
                >
                  <PlusIcon className="mr-1.5 h-3.5 w-3.5" />
                  Add
                </Button>
              )}
            </div>

            {subMembers.map((sm, idx) => (
              <div
                key={idx}
                className="rounded-xl border border-slate-200 bg-slate-50/60 p-4 space-y-3"
              >
                <div className="flex items-center justify-between">
                  <p className="text-xs font-semibold uppercase tracking-wide text-slate-400">
                    Sub-member {idx + 1}
                  </p>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="h-7 w-7 p-0 text-slate-400 hover:text-red-500"
                    onClick={() => removeSubMember(idx)}
                    disabled={submitting}
                  >
                    <Trash2Icon className="h-4 w-4" />
                  </Button>
                </div>
                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="space-y-1.5">
                    <Label htmlFor={`sm-name-${idx}`}>Name</Label>
                    <Input
                      id={`sm-name-${idx}`}
                      placeholder="Full name"
                      value={sm.name}
                      onChange={(e) => updateSubMember(idx, "name", e.target.value)}
                      disabled={submitting}
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor={`sm-relation-${idx}`}>Relation</Label>
                    <Input
                      id={`sm-relation-${idx}`}
                      placeholder="e.g. Spouse, Child"
                      value={sm.relation}
                      onChange={(e) => updateSubMember(idx, "relation", e.target.value)}
                      disabled={submitting}
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor={`sm-email-${idx}`}>Email</Label>
                    <Input
                      id={`sm-email-${idx}`}
                      type="email"
                      placeholder="email@example.com"
                      value={sm.email}
                      onChange={(e) => updateSubMember(idx, "email", e.target.value)}
                      disabled={submitting}
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor={`sm-phone-${idx}`}>Phone</Label>
                    <Input
                      id={`sm-phone-${idx}`}
                      placeholder="Mobile number"
                      value={sm.phone}
                      onChange={(e) => updateSubMember(idx, "phone", e.target.value)}
                      disabled={submitting}
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>

          <DialogFooter className="flex flex-col gap-2 sm:flex-row">
            <Button
              type="button"
              variant="ghost"
              className="text-slate-500"
              onClick={onSkip}
              disabled={submitting}
            >
              Skip for now
            </Button>
            <Button
              type="submit"
              disabled={submitting || !name.trim() || !phone.trim() || !address.trim()}
            >
              {submitting ? "Saving…" : "Save profile"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
