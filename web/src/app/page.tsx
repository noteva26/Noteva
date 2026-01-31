"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

// Admin root redirects to dashboard
export default function AdminRoot() {
  const router = useRouter();
  
  useEffect(() => {
    router.replace("/manage");
  }, [router]);
  
  return (
    <div className="flex h-screen items-center justify-center">
      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
    </div>
  );
}
