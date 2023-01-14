import CreateLayout, {
  CreateForm,
  CreateFormButtons,
  CreateFormFileUpload,
  CreateFormGroup,
  CreateFormRadioGroup,
  CreateFormSeparator,
} from '@/components/layouts/CreateLayout'
import { HTMLAttributes, useCallback, useMemo } from 'react'
import cx from 'classnames'
import { useAuth } from '@/components/organisms/AuthContext'
import {
  useCreate,
  createActionUpdatePost,
} from '@/components/organisms/CreateContext'
import { AccessType } from '@/core/api'
import { Formik } from 'formik'
import Head from 'next/head'
import { IoEarthOutline, IoHomeOutline, IoListOutline } from 'react-icons/io5'
import { useOrbits } from '@/components/organisms/OrbitContext'
import { useRouter } from 'next/router'
import AsidePlaceholder from '@/components/quarks/AsidePlaceholder'

interface IEditOrbitPostForm {
  title?: string
  content_md?: string
  content_warning?: string
}

export default function EditPostPage({
  className,
}: HTMLAttributes<HTMLDivElement>) {
  const { session } = useAuth()
  const { state, dispatch } = useCreate()
  const { submitting, defaultFormValues } = state
  const router = useRouter()
  const postId = router.query.postId as string | undefined

  const onSubmit: (values: IEditOrbitPostForm) => Promise<void> = useCallback(
    async (values) => {
      if (!postId) {
        return
      }

      if (submitting) {
        return
      }

      createActionUpdatePost(postId, values, session?.access_token, dispatch)
    },
    [submitting, session, postId]
  )

  return (
    <CreateLayout
      className={cx('orbit-page-new-post', className)}
      postId={postId}
    >
      <Head>
        <title>Orbit</title>
        <meta
          name="description"
          content="Welcome to Orbit, your place to share cool things with the world in an open, federated network"
        />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      {defaultFormValues && (
        <Formik initialValues={defaultFormValues} onSubmit={onSubmit}>
          <CreateForm title="Update Post">
            <CreateFormGroup
              title="Post Title"
              id="title"
              name="title"
              placeholder="Title"
              disabled={submitting}
            />
            <CreateFormGroup
              title="Content"
              id="content_md"
              name="content_md"
              placeholder="**Hello**, world!"
              as="textarea"
              disabled={submitting}
            />
            <CreateFormButtons
              submitTitle="Save Changes"
              cancelTitle="Cancel"
              disabled={submitting}
            />
          </CreateForm>
        </Formik>
      )}
      {!defaultFormValues && <AsidePlaceholder rows={10} />}
    </CreateLayout>
  )
}
