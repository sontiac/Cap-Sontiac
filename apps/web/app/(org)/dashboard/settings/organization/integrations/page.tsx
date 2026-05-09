import { getCurrentUser } from "@cap/database/auth/session";
import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { getOrganizationStorageSettings } from "@/actions/organization/storage";
import { OrganizationStorageIntegrations } from "./storage-integrations";

export const metadata: Metadata = {
	title: "Organization Integrations — Cap",
};

export default async function OrganizationIntegrationsPage() {
	const user = await getCurrentUser();

	if (!user) {
		redirect("/auth/signin");
	}

	if (!user.activeOrganizationId) {
		redirect("/dashboard/caps");
	}

	const settings = await getOrganizationStorageSettings(
		user.activeOrganizationId,
	);

	return <OrganizationStorageIntegrations initialSettings={settings} />;
}
