yazi_macro::mod_flat!(demand opendal read_dir rw_file stat);

pub(super) fn init() {
	::opendal::init_default_registry();
	opendal_http_transport_reqwest::install_default();
}
