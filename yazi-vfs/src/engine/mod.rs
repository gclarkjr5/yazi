yazi_macro::mod_pub!(lua opendal sftp);

yazi_macro::mod_flat!(absolute calculator copier dir_entry demand engine engines file_cursor read_dir rw_file);

pub(super) fn init() {
	opendal::init();
	sftp::init();
}
