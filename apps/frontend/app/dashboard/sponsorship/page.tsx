"use client";

/**
 * Sponsorship Management — /dashboard/sponsorship
 *
 * Two tabs:
 *   Sponsors — table with name, company, phone, email, total contributions
 *   Links    — table with token (truncated), sponsor, amount, purpose, status, date
 *
 * Features:
 *   - Add Sponsor dialog
 *   - Edit Sponsor dialog
 *   - Generate Link dialog per sponsor (or generic)
 *   - Deactivate link button
 *   - Copy link URL button
 */

import { useState } from "react";
import { useAuth } from "@/hooks/use-auth";
import {
  PlusIcon,
  PencilIcon,
  TrashIcon,
  LinkIcon,
  CopyIcon,
  CheckIcon,
  RefreshCwIcon,
  XCircleIcon,
  UsersIcon,
  BuildingIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { apiFetch } from "@/lib/api-client";
import { useApi } from "@/lib/hooks/use-api";
import { formatCurrency, formatDate, formatSponsorPurpose } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Sponsor {
  id: string;
  name: string;
  phone: string;
  email: string;
  company: string | null;
  createdAt: string;
  totalContributions: number;
}

interface SponsorLink {
  id: string;
  sponsorId: string | null;
  token: string;
  amount: string | null;
  upiId: string;
  isActive: boolean;
  expiresAt: string | null;
  sponsorPurpose: string;
  createdAt: string;
  sponsor: { id: string; name: string; company: string | null } | null;
  createdBy: { id: string; name: string };
  linkUrl: string;
}

interface PaginatedSponsors {
  data: Sponsor[];
  total: number;
  page: number;
  limit: number;
  totalPages: number;
}

interface PaginatedLinks {
  data: SponsorLink[];
  total: number;
  page: number;
  limit: number;
  totalPages: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SPONSOR_PURPOSES = [
  { value: "TITLE_SPONSOR", label: "Title Sponsor" },
  { value: "GOLD_SPONSOR", label: "Gold Sponsor" },
  { value: "SILVER_SPONSOR", label: "Silver Sponsor" },
  { value: "FOOD_PARTNER", label: "Food Partner" },
  { value: "MEDIA_PARTNER", label: "Media Partner" },
  { value: "STALL_VENDOR", label: "Stall Vendor" },
  { value: "MARKETING_PARTNER", label: "Marketing Partner" },
];

function purposeLabel(purpose: string): string {
  return SPONSOR_PURPOSES.find((p) => p.value === purpose)?.label ?? formatSponsorPurpose(purpose);
}

// formatCurrency and formatDate are imported from @/lib/utils

// ---------------------------------------------------------------------------
// Main Page Component
// ---------------------------------------------------------------------------

export default function SponsorshipPage() {
  const { user, loading: authLoading } = useAuth();
  const isAdmin = user?.role === "ADMIN";

  // ---- Sponsors state ----
  const [sponsorsPage, setSponsorsPage] = useState(1);
  const [sponsorsSearch, setSponsorsSearch] = useState("");

  // ---- Links state ----
  const [linksPage, setLinksPage] = useState(1);

  // ---- Dialog state ----
  const [showAddSponsor, setShowAddSponsor] = useState(false);
  const [showEditSponsor, setShowEditSponsor] = useState(false);
  const [editingSponsor, setEditingSponsor] = useState<Sponsor | null>(null);
  const [showGenerateLink, setShowGenerateLink] = useState(false);
  const [linkTargetSponsor, setLinkTargetSponsor] = useState<Sponsor | null>(null);
  const [generatedUrl, setGeneratedUrl] = useState<string | null>(null);
  const [copiedUrl, setCopiedUrl] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [formLoading, setFormLoading] = useState(false);

  // ---- Add/Edit sponsor form ----
  const [sponsorForm, setSponsorForm] = useState({
    name: "",
    phone: "",
    email: "",
    company: "",
  });

  // ---- Generate link form ----
  const [linkForm, setLinkForm] = useState({
    amount: "",
    upiId: "",
    bankAccountNumber: "",
    bankName: "",
    ifscCode: "",
    expiresAt: "",
    sponsorPurpose: "",
  });

  const LIMIT = 20;
  const sponsorParams = new URLSearchParams({
    page: String(sponsorsPage),
    limit: String(LIMIT),
    ...(sponsorsSearch ? { search: sponsorsSearch } : {}),
  });
  const linkParams = new URLSearchParams({
    page: String(linksPage),
    limit: String(LIMIT),
  });

  const {
    data: sponsorsResponse,
    isLoading: sponsorsLoadingRaw,
    mutate: mutateSponsors,
  } = useApi<PaginatedSponsors>(
    user ? `/api/sponsors?${sponsorParams.toString()}` : null,
    {
      dedupingInterval: 15_000,
      revalidateOnFocus: true,
      keepPreviousData: true,
    }
  );
  const {
    data: linksResponse,
    isLoading: linksLoadingRaw,
    mutate: mutateLinks,
  } = useApi<PaginatedLinks>(
    user ? `/api/sponsor-links?${linkParams.toString()}` : null,
    {
      dedupingInterval: 15_000,
      revalidateOnFocus: true,
      keepPreviousData: true,
    }
  );

  const sponsors = sponsorsResponse?.data ?? [];
  const sponsorsTotal = sponsorsResponse?.total ?? 0;
  const sponsorsLoading = authLoading || (sponsorsLoadingRaw && !sponsorsResponse);
  const links = linksResponse?.data ?? [];
  const linksTotal = linksResponse?.total ?? 0;
  const linksLoading = authLoading || (linksLoadingRaw && !linksResponse);

  // ---- Add sponsor ----
  async function handleAddSponsor() {
    setFormLoading(true);
    setFormError(null);
    try {
      const res = await apiFetch("/api/sponsors", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: sponsorForm.name,
          phone: sponsorForm.phone,
          email: sponsorForm.email,
          company: sponsorForm.company || null,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        const fieldLabels: Record<string, string> = { name: "Name", phone: "Phone", email: "Email", company: "Company" };
        setFormError(data?.details?.fieldErrors
          ? Object.entries(data.details.fieldErrors as Record<string, string[]>).map(([f, e]) => `${fieldLabels[f] ?? f}: ${e[0]}`).join(" • ")
          : (data.error ?? "Failed to create sponsor"));
        return;
      }
      setShowAddSponsor(false);
      setSponsorForm({ name: "", phone: "", email: "", company: "" });
      await mutateSponsors();
    } finally {
      setFormLoading(false);
    }
  }

  // ---- Edit sponsor ----
  function openEditSponsor(sponsor: Sponsor) {
    setEditingSponsor(sponsor);
    setSponsorForm({
      name: sponsor.name,
      phone: sponsor.phone,
      email: sponsor.email,
      company: sponsor.company ?? "",
    });
    setShowEditSponsor(true);
    setFormError(null);
  }

  async function handleEditSponsor() {
    if (!editingSponsor) return;
    setFormLoading(true);
    setFormError(null);
    try {
      const res = await apiFetch(`/api/sponsors/${editingSponsor.id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: sponsorForm.name,
          phone: sponsorForm.phone,
          email: sponsorForm.email,
          company: sponsorForm.company || null,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        const fieldLabels: Record<string, string> = { name: "Name", phone: "Phone", email: "Email", company: "Company" };
        setFormError(data?.details?.fieldErrors
          ? Object.entries(data.details.fieldErrors as Record<string, string[]>).map(([f, e]) => `${fieldLabels[f] ?? f}: ${e[0]}`).join(" • ")
          : (data.error ?? "Failed to update sponsor"));
        return;
      }
      setShowEditSponsor(false);
      await mutateSponsors();
    } finally {
      setFormLoading(false);
    }
  }

  // ---- Delete sponsor ----
  async function handleDeleteSponsor(sponsor: Sponsor) {
    if (!confirm(`Delete sponsor "${sponsor.name}"? This cannot be undone.`)) return;
    const res = await apiFetch(`/api/sponsors/${sponsor.id}`, { method: "DELETE" });
    const data = await res.json();
    if (!res.ok) {
      alert(data.error ?? "Failed to delete sponsor");
      return;
    }
    await mutateSponsors();
  }

  // ---- Open generate link dialog ----
  function openGenerateLink(sponsor: Sponsor | null) {
    setLinkTargetSponsor(sponsor);
    setLinkForm({
      amount: "",
      upiId: "",
      bankAccountNumber: "",
      bankName: "",
      ifscCode: "",
      expiresAt: "",
      sponsorPurpose: "",
    });
    setGeneratedUrl(null);
    setCopiedUrl(false);
    setFormError(null);
    setShowGenerateLink(true);
  }

  // ---- Generate link ----
  async function handleGenerateLink() {
    setFormLoading(true);
    setFormError(null);
    try {
      const bankDetails =
        linkForm.bankAccountNumber || linkForm.bankName || linkForm.ifscCode
          ? {
              accountNumber: linkForm.bankAccountNumber || undefined,
              bankName: linkForm.bankName || undefined,
              ifscCode: linkForm.ifscCode || undefined,
            }
          : null;

      const body: Record<string, unknown> = {
        sponsorId: linkTargetSponsor?.id ?? null,
        upiId: linkForm.upiId,
        sponsorPurpose: linkForm.sponsorPurpose,
        ...(linkForm.amount ? { amount: parseFloat(linkForm.amount) } : {}),
        ...(bankDetails ? { bankDetails } : {}),
        ...(linkForm.expiresAt ? { expiresAt: linkForm.expiresAt } : {}),
      };

      const res = await apiFetch("/api/sponsor-links", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) {
        setFormError(data.error ?? "Failed to generate link");
        return;
      }
      setGeneratedUrl(data.url);
      await mutateLinks();
    } finally {
      setFormLoading(false);
    }
  }

  // ---- Copy URL ----
  async function copyUrl(url: string) {
    await navigator.clipboard.writeText(url);
    setCopiedUrl(true);
    setTimeout(() => setCopiedUrl(false), 2000);
  }

  // ---- Deactivate link ----
  async function handleDeactivateLink(link: SponsorLink) {
    if (!confirm("Deactivate this sponsor link?")) return;
    const res = await apiFetch(`/api/sponsor-links/${link.token}`, { method: "PATCH" });
    const data = await res.json();
    if (!res.ok) {
      alert(data.error ?? "Failed to deactivate link");
      return;
    }
    await mutateLinks();
  }

  const totalContributions = sponsors.reduce((s, sp) => s + sp.totalContributions, 0);

  return (
    <div className="space-y-6 p-4 sm:p-6">
      {/* Header */}
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">Sponsorship Management</h1>
          <p className="mt-1 text-sm text-slate-500">
            Manage sponsors and generate payment links
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            onClick={() => openGenerateLink(null)}
            variant="outline"
            className="gap-2"
          >
            <LinkIcon className="h-4 w-4" />
            Generic Link
          </Button>
          <Button
            onClick={() => { setSponsorForm({ name: "", phone: "", email: "", company: "" }); setFormError(null); setShowAddSponsor(true); }}
            className="gap-2"
          >
            <PlusIcon className="h-4 w-4" />
            Add Sponsor
          </Button>
        </div>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm font-medium text-slate-500">
              <UsersIcon className="h-4 w-4" /> Total Sponsors
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{sponsorsTotal}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm font-medium text-slate-500">
              <BuildingIcon className="h-4 w-4" /> Total Contributions
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-emerald-600">
              {formatCurrency(totalContributions)}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm font-medium text-slate-500">
              <LinkIcon className="h-4 w-4" /> Active Links
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-sky-600">
              {links.filter((l) => l.isActive).length}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Tabs */}
      <Tabs defaultValue="sponsors">
        <TabsList className="grid h-auto w-full grid-cols-1 gap-2 sm:inline-flex sm:h-12 sm:w-auto sm:grid-cols-none">
          <TabsTrigger value="sponsors">Sponsors ({sponsorsTotal})</TabsTrigger>
          <TabsTrigger value="links">Payment Links ({linksTotal})</TabsTrigger>
        </TabsList>

        {/* ---- Sponsors tab ---- */}
        <TabsContent value="sponsors" className="mt-4">
          <div className="mb-4 flex flex-col gap-3 sm:flex-row">
            <Input
              placeholder="Search by name, email, or company..."
              value={sponsorsSearch}
              onChange={(e) => { setSponsorsSearch(e.target.value); setSponsorsPage(1); }}
              className="w-full sm:max-w-xs"
            />
            <Button variant="outline" size="icon" onClick={() => void mutateSponsors()}>
              <RefreshCwIcon className="h-4 w-4" />
            </Button>
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Company</TableHead>
                  <TableHead>Phone</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead className="text-right">Total Contributions</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {sponsorsLoading ? (
                  <TableRow>
                    <TableCell colSpan={6} className="py-8 text-center text-slate-500">
                      Loading...
                    </TableCell>
                  </TableRow>
                ) : sponsors.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={6} className="py-8 text-center text-slate-500">
                      No sponsors found
                    </TableCell>
                  </TableRow>
                ) : (
                  sponsors.map((sponsor) => (
                    <TableRow key={sponsor.id}>
                      <TableCell className="font-medium">{sponsor.name}</TableCell>
                      <TableCell>{sponsor.company ?? <span className="text-slate-400">—</span>}</TableCell>
                      <TableCell className="font-mono text-sm">{sponsor.phone}</TableCell>
                      <TableCell>{sponsor.email}</TableCell>
                      <TableCell className="text-right font-semibold">
                        {sponsor.totalContributions > 0
                          ? <span className="text-emerald-700">{formatCurrency(sponsor.totalContributions)}</span>
                          : <span className="text-slate-400">—</span>
                        }
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex items-center justify-end gap-1">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => openGenerateLink(sponsor)}
                            className="gap-1 text-xs"
                          >
                            <LinkIcon className="h-3 w-3" />
                            Link
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => openEditSponsor(sponsor)}
                            title="Edit"
                          >
                            <PencilIcon className="h-4 w-4" />
                          </Button>
                          {isAdmin && (
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={() => handleDeleteSponsor(sponsor)}
                              title="Delete"
                              className="text-rose-500 hover:text-rose-700"
                            >
                              <TrashIcon className="h-4 w-4" />
                            </Button>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>

          {/* Sponsors pagination */}
          {Math.ceil(sponsorsTotal / LIMIT) > 1 && (
            <div className="flex items-center justify-between mt-4">
              <p className="text-sm text-slate-500">
                Showing {(sponsorsPage - 1) * LIMIT + 1}–{Math.min(sponsorsPage * LIMIT, sponsorsTotal)} of {sponsorsTotal}
              </p>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={sponsorsPage <= 1}
                  onClick={() => setSponsorsPage((p) => p - 1)}
                >
                  Previous
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={sponsorsPage >= Math.ceil(sponsorsTotal / LIMIT)}
                  onClick={() => setSponsorsPage((p) => p + 1)}
                >
                  Next
                </Button>
              </div>
            </div>
          )}
        </TabsContent>

        {/* ---- Links tab ---- */}
        <TabsContent value="links" className="mt-4">
          <div className="flex gap-3 mb-4">
            <Button variant="outline" size="icon" onClick={() => void mutateLinks()}>
              <RefreshCwIcon className="h-4 w-4" />
            </Button>
          </div>

          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Token</TableHead>
                  <TableHead>Sponsor</TableHead>
                  <TableHead>Purpose</TableHead>
                  <TableHead className="text-right">Amount</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Expires</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {linksLoading ? (
                  <TableRow>
                    <TableCell colSpan={8} className="py-8 text-center text-slate-500">
                      Loading...
                    </TableCell>
                  </TableRow>
                ) : links.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={8} className="py-8 text-center text-slate-500">
                      No sponsor links found
                    </TableCell>
                  </TableRow>
                ) : (
                  links.map((link) => {
                    const isExpired =
                      link.expiresAt != null &&
                      new Date(link.expiresAt) < new Date();
                    const statusBadge = !link.isActive ? (
                      <Badge variant="outline" className="text-slate-500">Inactive</Badge>
                    ) : isExpired ? (
                      <Badge variant="outline" className="border-amber-300 text-amber-700">Expired</Badge>
                    ) : (
                      <Badge className="bg-emerald-100 text-emerald-700 hover:bg-emerald-100">Active</Badge>
                    );

                    return (
                      <TableRow key={link.id}>
                        <TableCell className="font-mono text-xs text-slate-600">
                          {link.token.substring(0, 8)}...
                        </TableCell>
                        <TableCell>
                          {link.sponsor ? (
                            <div>
                              <div className="font-medium text-sm">{link.sponsor.name}</div>
                              {link.sponsor.company && (
                                <div className="text-xs text-slate-500">{link.sponsor.company}</div>
                              )}
                            </div>
                          ) : (
                            <span className="text-sm text-slate-400">Generic</span>
                          )}
                        </TableCell>
                        <TableCell>
                          <span className="text-sm">{purposeLabel(link.sponsorPurpose)}</span>
                        </TableCell>
                        <TableCell className="text-right">
                          {link.amount
                            ? formatCurrency(Number(link.amount))
                            : <span className="text-sm text-slate-400">Open</span>
                          }
                        </TableCell>
                        <TableCell>{statusBadge}</TableCell>
                        <TableCell className="text-sm">
                          {link.expiresAt ? formatDate(link.expiresAt) : <span className="text-slate-400">—</span>}
                        </TableCell>
                        <TableCell className="text-sm text-slate-500">
                          {formatDate(link.createdAt)}
                        </TableCell>
                        <TableCell className="text-right">
                          <div className="flex items-center justify-end gap-1">
                            <Button
                              variant="ghost"
                              size="icon"
                              title="Copy link"
                              onClick={() => copyUrl(link.linkUrl)}
                            >
                              <CopyIcon className="h-4 w-4" />
                            </Button>
                            {link.isActive && !isExpired && (
                              <Button
                                variant="ghost"
                                size="icon"
                                title="Deactivate"
                                onClick={() => handleDeactivateLink(link)}
                                className="text-rose-500 hover:text-rose-700"
                              >
                                <XCircleIcon className="h-4 w-4" />
                              </Button>
                            )}
                          </div>
                        </TableCell>
                      </TableRow>
                    );
                  })
                )}
              </TableBody>
            </Table>
          </div>

          {/* Links pagination */}
          {Math.ceil(linksTotal / LIMIT) > 1 && (
            <div className="flex items-center justify-between mt-4">
              <p className="text-sm text-slate-500">
                Showing {(linksPage - 1) * LIMIT + 1}–{Math.min(linksPage * LIMIT, linksTotal)} of {linksTotal}
              </p>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={linksPage <= 1}
                  onClick={() => setLinksPage((p) => p - 1)}
                >
                  Previous
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={linksPage >= Math.ceil(linksTotal / LIMIT)}
                  onClick={() => setLinksPage((p) => p + 1)}
                >
                  Next
                </Button>
              </div>
            </div>
          )}
        </TabsContent>
      </Tabs>

      {/* ====================== DIALOGS ====================== */}

      {/* Add Sponsor Dialog */}
      <Dialog open={showAddSponsor} onOpenChange={setShowAddSponsor}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Sponsor</DialogTitle>
            <DialogDescription>Enter the sponsor&apos;s details to register them in the system.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            {formError && (
              <div className="rounded-xl border border-rose-200 bg-rose-50 p-2 text-sm text-rose-600">
                {formError}
              </div>
            )}
            <div className="space-y-2">
              <Label htmlFor="s-name">Name *</Label>
              <Input
                id="s-name"
                value={sponsorForm.name}
                onChange={(e) => setSponsorForm((f) => ({ ...f, name: e.target.value }))}
                placeholder="Full name"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="s-company">Company</Label>
              <Input
                id="s-company"
                value={sponsorForm.company}
                onChange={(e) => setSponsorForm((f) => ({ ...f, company: e.target.value }))}
                placeholder="Company name (optional)"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="s-phone">Phone (WhatsApp) *</Label>
              <Input
                id="s-phone"
                value={sponsorForm.phone}
                onChange={(e) => setSponsorForm((f) => ({ ...f, phone: e.target.value }))}
                placeholder="+919876543210"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="s-email">Email *</Label>
              <Input
                id="s-email"
                type="email"
                value={sponsorForm.email}
                onChange={(e) => setSponsorForm((f) => ({ ...f, email: e.target.value }))}
                placeholder="email@example.com"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddSponsor(false)}>
              Cancel
            </Button>
            <Button onClick={handleAddSponsor} disabled={formLoading}>
              {formLoading ? "Creating..." : "Create Sponsor"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Sponsor Dialog */}
      <Dialog open={showEditSponsor} onOpenChange={setShowEditSponsor}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Edit Sponsor</DialogTitle>
            <DialogDescription>Update the sponsor&apos;s contact and company details.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            {formError && (
              <div className="rounded-xl border border-rose-200 bg-rose-50 p-2 text-sm text-rose-600">
                {formError}
              </div>
            )}
            <div className="space-y-2">
              <Label htmlFor="es-name">Name *</Label>
              <Input
                id="es-name"
                value={sponsorForm.name}
                onChange={(e) => setSponsorForm((f) => ({ ...f, name: e.target.value }))}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="es-company">Company</Label>
              <Input
                id="es-company"
                value={sponsorForm.company}
                onChange={(e) => setSponsorForm((f) => ({ ...f, company: e.target.value }))}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="es-phone">Phone *</Label>
              <Input
                id="es-phone"
                value={sponsorForm.phone}
                onChange={(e) => setSponsorForm((f) => ({ ...f, phone: e.target.value }))}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="es-email">Email *</Label>
              <Input
                id="es-email"
                type="email"
                value={sponsorForm.email}
                onChange={(e) => setSponsorForm((f) => ({ ...f, email: e.target.value }))}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowEditSponsor(false)}>
              Cancel
            </Button>
            <Button onClick={handleEditSponsor} disabled={formLoading}>
              {formLoading ? "Saving..." : "Save Changes"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Generate Link Dialog */}
      <Dialog open={showGenerateLink} onOpenChange={(open) => { if (!open) setGeneratedUrl(null); setShowGenerateLink(open); }}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {linkTargetSponsor
                ? `Generate Payment Link — ${linkTargetSponsor.name}`
                : "Generate Generic Payment Link"}
            </DialogTitle>
            <DialogDescription>
              {linkTargetSponsor
                ? "Configure and generate a Razorpay payment link for this sponsor."
                : "Generate a generic Razorpay payment link for new sponsors."}
            </DialogDescription>
          </DialogHeader>

          {generatedUrl ? (
            /* ---- Success view after link generated ---- */
            <div className="space-y-4 py-2">
              <div className="rounded-xl border border-emerald-200 bg-emerald-50 p-3 text-sm font-medium text-emerald-700">
                Payment link generated successfully!
              </div>
              <div className="space-y-2">
                <Label>Sponsor Payment Link</Label>
                <div className="flex items-center gap-2">
                  <Input
                    readOnly
                    value={generatedUrl}
                    className="bg-slate-50 font-mono text-sm"
                  />
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => copyUrl(generatedUrl)}
                    title="Copy"
                  >
                    {copiedUrl ? <CheckIcon className="h-4 w-4 text-emerald-600" /> : <CopyIcon className="h-4 w-4" />}
                  </Button>
                </div>
                <p className="text-xs text-slate-500">
                  Share this link with the sponsor. They can pay via UPI or bank transfer.
                </p>
              </div>
            </div>
          ) : (
            /* ---- Form view ---- */
            <div className="space-y-4 py-2">
              {formError && (
                <div className="rounded-xl border border-rose-200 bg-rose-50 p-2 text-sm text-rose-600">
                  {formError}
                </div>
              )}

              {/* Sponsor Purpose */}
              <div className="space-y-2">
                <Label>Sponsorship Type *</Label>
                <Select
                  value={linkForm.sponsorPurpose}
                  onValueChange={(v) => setLinkForm((f) => ({ ...f, sponsorPurpose: v }))}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select sponsorship type..." />
                  </SelectTrigger>
                  <SelectContent>
                    {SPONSOR_PURPOSES.map((p) => (
                      <SelectItem key={p.value} value={p.value}>
                        {p.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* Amount */}
              <div className="space-y-2">
                <Label htmlFor="l-amount">Amount (₹) — leave blank for open-ended</Label>
                <Input
                  id="l-amount"
                  type="number"
                  min="1"
                  step="0.01"
                  value={linkForm.amount}
                  onChange={(e) => setLinkForm((f) => ({ ...f, amount: e.target.value }))}
                  placeholder="e.g. 50000 (optional)"
                />
              </div>

              {/* UPI ID */}
              <div className="space-y-2">
                <Label htmlFor="l-upi">UPI ID (Receiving) *</Label>
                <Input
                  id="l-upi"
                  value={linkForm.upiId}
                  onChange={(e) => setLinkForm((f) => ({ ...f, upiId: e.target.value }))}
                  placeholder="clubtreasurer@upi"
                />
              </div>

              {/* Bank Details (optional section) */}
              <div className="space-y-3 rounded-xl border border-slate-200 bg-slate-50 p-3">
                <p className="text-xs font-medium uppercase tracking-wide text-slate-600">
                  Bank Transfer Details (optional)
                </p>
                <div className="grid grid-cols-2 gap-2">
                  <div className="space-y-1">
                    <Label htmlFor="l-accno" className="text-xs">Account Number</Label>
                    <Input
                      id="l-accno"
                      value={linkForm.bankAccountNumber}
                      onChange={(e) => setLinkForm((f) => ({ ...f, bankAccountNumber: e.target.value }))}
                      placeholder="XXXXXXXXXXXX"
                      className="text-sm"
                    />
                  </div>
                  <div className="space-y-1">
                    <Label htmlFor="l-ifsc" className="text-xs">IFSC Code</Label>
                    <Input
                      id="l-ifsc"
                      value={linkForm.ifscCode}
                      onChange={(e) => setLinkForm((f) => ({ ...f, ifscCode: e.target.value.toUpperCase() }))}
                      placeholder="SBIN0001234"
                      className="text-sm font-mono"
                    />
                  </div>
                </div>
                <div className="space-y-1">
                  <Label htmlFor="l-bank" className="text-xs">Bank Name</Label>
                  <Input
                    id="l-bank"
                    value={linkForm.bankName}
                    onChange={(e) => setLinkForm((f) => ({ ...f, bankName: e.target.value }))}
                    placeholder="e.g. State Bank of India"
                    className="text-sm"
                  />
                </div>
              </div>

              {/* Expiry */}
              <div className="space-y-2">
                <Label htmlFor="l-exp">Expiry Date (optional)</Label>
                <Input
                  id="l-exp"
                  type="date"
                  value={linkForm.expiresAt}
                  onChange={(e) => setLinkForm((f) => ({ ...f, expiresAt: e.target.value }))}
                />
              </div>
            </div>
          )}

          <DialogFooter>
            {generatedUrl ? (
              <Button onClick={() => { setShowGenerateLink(false); setGeneratedUrl(null); }}>
                Done
              </Button>
            ) : (
              <>
                <Button variant="outline" onClick={() => setShowGenerateLink(false)}>
                  Cancel
                </Button>
                <Button
                  onClick={handleGenerateLink}
                  disabled={formLoading || !linkForm.upiId || !linkForm.sponsorPurpose}
                >
                  {formLoading ? "Generating..." : "Generate Link"}
                </Button>
              </>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
