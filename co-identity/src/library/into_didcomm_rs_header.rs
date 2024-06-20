pub fn into_didcomm_rs_header(value: crate::DidCommHeader) -> didcomm_rs::DidCommHeader {
	let mut result = didcomm_rs::DidCommHeader::default();
	result.id = value.id;
	result.m_type = value.message_type;
	result.to = value.to.into_iter().collect();
	result.from = value.from;
	result.thid = value.thid;
	result.pthid = value.pthid;
	result.created_time = value.created_time;
	result.expires_time = value.expires_time;
	result
}

pub fn from_didcomm_rs_header(value: didcomm_rs::DidCommHeader) -> crate::DidCommHeader {
	crate::DidCommHeader {
		id: value.id,
		message_type: value.m_type,
		to: value.to.into_iter().collect(),
		from: value.from,
		thid: value.thid,
		pthid: value.pthid,
		created_time: value.created_time,
		expires_time: value.expires_time,
	}
}
