import { PUBLIC_API_BASE_URL } from '$env/static/public';

const DEFAULT_API_BASE_URL = 'http://localhost:3000';

export const API_BASE_URL = (PUBLIC_API_BASE_URL || DEFAULT_API_BASE_URL).replace(/\/+$/, '');

export interface ApiFetchOptions extends RequestInit {
  token?: string | null;
}

export function buildApiUrl(path: string): string {
  if (/^https?:\/\//.test(path)) {
    return path;
  }

  return `${API_BASE_URL}${path.startsWith('/') ? path : `/${path}`}`;
}

export function apiFetch(path: string, options: ApiFetchOptions = {}): Promise<Response> {
  const { token, headers, body, ...rest } = options;
  const mergedHeaders = new Headers(headers ?? {});

  if (token) {
    mergedHeaders.set('Authorization', `Bearer ${token}`);
  }

  const hasBody = body !== undefined && body !== null;
  const isFormData = typeof FormData !== 'undefined' && body instanceof FormData;

  if (hasBody && !isFormData && !mergedHeaders.has('Content-Type')) {
    mergedHeaders.set('Content-Type', 'application/json');
  }

  return fetch(buildApiUrl(path), {
    ...rest,
    body,
    headers: mergedHeaders
  });
}
