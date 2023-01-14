import { HTMLProps } from 'react'
import cx from 'classnames'
import dayjs from 'dayjs'
import dayjsUtc from 'dayjs/plugin/utc'
import dayjsRelative from 'dayjs/plugin/relativeTime'

dayjs.extend(dayjsUtc)
dayjs.extend(dayjsRelative)

export default function FeedAlertCard({
  children,
  className,
  title,
  ...rest
}: HTMLProps<HTMLDivElement>) {
  return (
    <div className={cx('orbit-feed-alert-card', className)} {...rest}>
      {!!title && <div className="orbit-feed-alert-card__title">{title}</div>}
      <div className={cx('orbit-feed-alert-card__content')}>{children}</div>
    </div>
  )
}
