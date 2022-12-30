import {
  profileActionLoadProfile,
  useProfile,
} from '@/components/organisms/ProfileContext'
import { useEffect } from 'react'
import { useAuth } from '@/components/organisms/AuthContext'
import {
  feedActionLoadFeed,
  feedActionReset,
  FeedType,
  useFeed,
} from '@/components/organisms/FeedContext'
import { useRouter } from 'next/router'
import { postActionDismissPost, usePost } from './PostContext'
import {
  orbitActionClearOrbit,
  orbitActionLoadUserOrbits,
  useOrbits,
} from './OrbitContext'

export default function DefaultActionsDelegator() {
  const { session } = useAuth()
  const { state: profileState, dispatch: profileDispatch } = useProfile()
  const { state: feedState, dispatch: feedDispatch } = useFeed()
  const { state: postState, dispatch: postDispatch } = usePost()
  const { state: orbitsState, dispatch: orbitsDispatch } = useOrbits()
  const { route, query } = useRouter()

  useEffect(() => {
    if (
      !session ||
      profileState.profile ||
      profileState.loading ||
      profileState.loadingFailed
    ) {
      return
    }

    profileActionLoadProfile(session.access_token, profileDispatch)
  }, [session, profileState, profileDispatch])

  useEffect(() => {
    if (
      !feedState.initialLoadComplete ||
      feedState.loading ||
      feedState.loadingFailed
    ) {
      return
    }

    if (
      route === '/' &&
      feedState.type !== FeedType.PublicFederated &&
      feedState.type !== FeedType.Own
    ) {
      feedActionReset(feedDispatch)
    }

    if (
      route === '/orbits/[orbitShortcode]' &&
      feedState.type !== FeedType.Orbit
    ) {
      feedActionReset(feedDispatch)
    }
  }, [feedState, feedDispatch, route])

  useEffect(() => {
    if (route !== '/') {
      return
    }

    if (feedState.initialLoadComplete) {
      if (feedState.type === FeedType.PublicFederated && !session) {
        return
      } else if (feedState.type === FeedType.Own && session) {
        return
      } else if (
        feedState.type !== FeedType.PublicFederated &&
        feedState.type !== FeedType.Own
      ) {
        return
      }
    }

    if (feedState.loading || feedState.loadingFailed) {
      return
    }

    feedActionLoadFeed(0, session?.access_token, undefined, feedDispatch)
  }, [session, feedState, feedDispatch, route])

  useEffect(() => {
    if (route !== '/orbits/[orbitShortcode]' || !orbitsState.orbit) {
      return
    }

    if (feedState.initialLoadComplete) {
      return
    }

    if (feedState.loading || feedState.loadingFailed) {
      return
    }

    feedActionLoadFeed(
      0,
      session?.access_token,
      orbitsState.orbit,
      feedDispatch
    )
  }, [session, feedState, feedDispatch, orbitsState, route])

  useEffect(() => {
    if (!profileState.profile?.handle || !session || orbitsState.orbits) {
      return
    }

    if (orbitsState.loading || orbitsState.loadingFailed) {
      return
    }

    orbitActionLoadUserOrbits(
      profileState.profile.handle,
      session.access_token,
      orbitsDispatch
    )
  }, [session, profileState, orbitsState, orbitsDispatch])

  useEffect(() => {
    if (route !== '/feed/[postId]' && !!postState.post) {
      postActionDismissPost(postDispatch)
    }
  }, [postState, postDispatch, route, query])

  return <></>
}
