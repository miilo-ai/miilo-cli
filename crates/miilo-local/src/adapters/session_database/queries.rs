// Extract only assistant-visible text and bounded tool names, never raw JSON payloads.
pub const HERMES: &str = "SELECT cast(id AS TEXT),cast(timestamp*1000 AS INTEGER),substr(content,1,280),CASE WHEN finish_reason IS NULL THEN 'active' ELSE 'waiting' END FROM messages WHERE session_id=?1 AND role='assistant' AND active=1 ORDER BY timestamp DESC,id DESC LIMIT 128";
pub const OPENCODE: &str = "SELECT p.id,p.time_updated,
 CASE json_extract(p.data,'$.type') WHEN 'text' THEN substr(json_extract(p.data,'$.text'),1,280) WHEN 'tool' THEN 'Tool: ' || substr(json_extract(p.data,'$.tool'),1,80) END,
 CASE WHEN json_extract(m.data,'$.time.completed') IS NOT NULL THEN 'waiting' ELSE 'active' END
 FROM part p JOIN message m ON m.id=p.message_id AND m.session_id=p.session_id
 WHERE p.session_id=?1 AND json_extract(m.data,'$.role')='assistant'
 AND json_extract(p.data,'$.type') IN ('text','tool') ORDER BY p.time_updated DESC,p.id DESC LIMIT 128";
pub const OPENCODE_ERRORS: &str = "SELECT id,time_updated,'Agent reported an error','waiting' FROM message
 WHERE session_id=?1 AND json_extract(data,'$.role')='assistant' AND json_extract(data,'$.error.name') IS NOT NULL
 ORDER BY time_updated DESC LIMIT 1";
pub const DEVIN: &str = "WITH RECURSIVE chain(node_id,parent_node_id,chat_message,created_at,depth) AS (
 SELECT n.node_id,n.parent_node_id,n.chat_message,n.created_at,0 FROM message_nodes n JOIN sessions s ON s.id=n.session_id AND s.main_chain_id=n.node_id WHERE n.session_id=?1
 UNION ALL SELECT n.node_id,n.parent_node_id,n.chat_message,n.created_at,c.depth+1 FROM message_nodes n JOIN chain c ON n.node_id=c.parent_node_id WHERE n.session_id=?1 AND c.depth<127
 ) SELECT cast(node_id AS TEXT),created_at,
 CASE WHEN json_type(chat_message,'$.content')='text' THEN substr(json_extract(chat_message,'$.content'),1,280) END,
 CASE WHEN json_extract(chat_message,'$.metadata.finish_reason') IS NOT NULL THEN 'waiting' ELSE 'active' END
 FROM chain WHERE json_extract(chat_message,'$.role')='assistant' ORDER BY created_at DESC,node_id DESC LIMIT 128";
