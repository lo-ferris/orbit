SELECT DISTINCT 'post' as event_type, p.*, u.handle AS user_handle, u.fediverse_id AS user_fediverse_id, 
u.fediverse_uri AS user_fediverse_uri, u.avatar_url AS user_avatar_url, COUNT(DISTINCT l.like_id) AS likes, FALSE AS liked,
COUNT(DISTINCT c.comment_id) AS comments, u.handle AS event_user_handle, u.fediverse_id AS event_user_fediverse_id, 
u.fediverse_uri AS event_user_fediverse_uri, u.avatar_url AS event_user_avatar_url, 
ob.name as orbit_name, ob.shortcode as orbit_shortcode, ob.uri as orbit_uri, ob.fediverse_uri as orbit_fediverse_uri, ob.avatar_uri as orbit_avatar_uri, 
pa.attachment_id, pa.user_id as attachment_user_id,  pa.post_id as attachment_post_id, pa.uri as attachment_uri, pa.width as attachment_width, 
pa.height as attachment_height, pa.content_type as attachment_content_type, pa.storage_ref as attachment_storage_ref, 
pa.blurhash as attachment_blurhash, pa.created_at as attachment_created_at
FROM posts p
INNER JOIN users u
ON u.user_id = p.user_id
LEFT OUTER JOIN likes l
ON l.post_id = p.post_id
LEFT OUTER JOIN comments c
ON c.post_id = p.post_id
LEFT OUTER JOIN post_attachments pa
ON pa.post_id = p.post_id
LEFT OUTER JOIN orbits ob
ON ob.orbit_id = p.orbit_id
WHERE p.visibility IN ('public_federated', 'public_local')
AND ob.orbit_id = $1
GROUP BY p.post_id, u.user_id, pa.attachment_id, ob.orbit_id
ORDER BY p.created_at DESC
LIMIT $2
OFFSET $3
