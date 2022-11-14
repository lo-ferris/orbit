import Config from './config'

function buildDefaultHeaders(authToken: string): any {
  return {
    headers: {
      Authorization: `Bearer ${authToken}`,
      Accept: 'application/json',
    },
    mode: 'cors',
    credentials: 'include',
  } as any
}

function buildUnauthenticatedHeaders(): any {
  return {
    headers: {
      Accept: 'application/json',
    },
    mode: 'cors',
  } as any
}

export interface IListResponse<T> {
  data: T[]
  page: number
  total_items: number
  total_pages: number
}

export interface IProfile {
  user_id: string
  fediverse_id: string
  handle?: string
  avatar_url?: string
  email?: string
}

export enum AccessType {
  Unknown = 'unknown',
  Shadow = 'shadow',
  Unlisted = 'unlisted',
  Private = 'private',
  FollowersOnly = 'followers_only',
  PublicLocal = 'public_local',
  PublicFederated = 'public_federated',
}

export interface IPost {
  post_id: string
  user_id: string
  user_handle: string
  user_fediverse_id: string
  user_avatar_url?: string
  uri: string
  content_md: string
  content_html: string
  content_image_uri_small?: string
  content_image_uri_medium?: string
  content_image_uri_large?: string
  content_width_small?: number
  content_width_medium?: number
  content_width_large?: number
  content_height_small?: number
  content_height_medium?: number
  content_height_large?: number
  content_type_small?: string
  content_type_medium?: string
  content_type_large?: string
  visibility: AccessType
  created_at: number
  updated_at: number
}

export async function fetchProfile(authToken: string): Promise<IProfile> {
  const response = await fetch(`${Config.apiUri}/profile`, {
    ...buildDefaultHeaders(authToken),
    method: 'GET',
  })

  if (response.status !== 200) {
    throw new Error('Request failed')
  }

  return await response.json()
}

export async function fetchFederatedFeed(
  page: number,
  pageSize: number = 20
): Promise<IListResponse<IPost>> {
  const response = await fetch(
    `${Config.apiUri}/feed/federated?page=${page}&page_size=${pageSize}`,
    {
      ...buildUnauthenticatedHeaders(),
      method: 'GET',
    }
  )

  if (response.status !== 200) {
    throw new Error('Request failed')
  }

  return await response.json()
}

export async function fetchOwnFeed(
  authToken: string,
  page: number,
  pageSize: number = 20
): Promise<IListResponse<IPost>> {
  const response = await fetch(
    `${Config.apiUri}/feed?page=${page}&page_size=${pageSize}`,
    {
      ...buildDefaultHeaders(authToken),
      method: 'GET',
    }
  )

  if (response.status !== 200) {
    throw new Error('Request failed')
  }

  return await response.json()
}