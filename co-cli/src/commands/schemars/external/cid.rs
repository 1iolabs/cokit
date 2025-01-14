/// Cid doesn't derive the JsonSchema macro. Therefore we use a newtype struct CoCid and manually link to a cid.json
/// schema. This function generates the content that that file should contain.
pub fn generate_cid_schema() -> String {
	"{
        \"$schema\": \"http://json-schema.org/draft-07/schema#\",
        \"title\": \"Cid\",
        \"type\": \"object\",
        \"required\": [
            \"/\"
        ],
        \"properties\": {
            \"/\": {
                \"type\": \"string\"
            }
        }
    }"
	.to_owned()
}
