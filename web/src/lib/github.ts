export function parseGitHubRepo(input: string) {
  const trimmed = input.trim();
  if (!trimmed) return "";

  const match = trimmed.match(/github\.com\/([^/]+\/[^/#?]+)/);
  return (match?.[1] || trimmed).replace(/\.git$/, "");
}
