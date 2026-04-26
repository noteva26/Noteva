interface ApiErrorShape {
  response?: {
    data?: {
      error?: {
        message?: string;
      };
    };
  };
}

export function getApiErrorMessage(error: unknown, fallback: string) {
  if (typeof error !== "object" || error === null || !("response" in error)) {
    return fallback;
  }

  const response = (error as ApiErrorShape).response;
  return response?.data?.error?.message || fallback;
}
