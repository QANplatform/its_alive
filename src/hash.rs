pub const SWIFFTX_INPUT_BLOCK_SIZE : u32 = 256 ;
pub const SWIFFTX_OUTPUT_BLOCK_SIZE : u32 = 65 ;
pub const HAIFA_SALT_SIZE : u32 = 8 ;
pub const HAIFA_NUM_OF_BITS_SIZE : u32 = 8 ;
pub const HAIFA_INPUT_BLOCK_SIZE : u32 = 175 ;
pub const SUCCESS : u32 = 0 ;
pub const FAIL : u32 = 1 ;
pub const BAD_HASHBITLEN : u32 = 2 ;
pub const BAD_SALT_SIZE : u32 = 3 ;
pub const SET_SALT_VALUE_FAILED : u32 = 4 ;
pub const INPUT_DATA_NOT_ALIGNED : u32 = 5 ;
pub const HAIFA_IV : u32 = 0 ;
pub type wchar_t = ::std::os::raw::c_int;
#[repr(C)] 
#[repr(align(16))] 
#[derive( Debug, Copy, Clone)] 
pub struct max_align_t { 
    pub __clang_max_align_nonce1 : ::std::os::raw::c_longlong, 
    pub __bindgen_padding_0 : u64, 
    pub __clang_max_align_nonce2 : u128, 
} 
#[test] 
fn bindgen_test_layout_max_align_t ( ) { 
    assert_eq!(::std::mem::size_of::<max_align_t>(), 32usize, concat!("Size of: ", stringify!(max_align_t)));
    assert_eq!(::std::mem::align_of::<max_align_t>() , 16usize , concat!( "Alignment of " , stringify!(max_align_t))) ;
    assert_eq!(unsafe {&(*(::std::ptr::null::<max_align_t>())).__clang_max_align_nonce1 as * const _ as usize } , 0usize , concat!("Offset of field: ", stringify!(max_align_t), "::", stringify!(__clang_max_align_nonce1)));
    assert_eq!(unsafe {&(*(::std::ptr::null::<max_align_t>())).__clang_max_align_nonce2 as * const _ as usize } , 16usize , concat!("Offset of field: ", stringify!(max_align_t), "::", stringify!(__clang_max_align_nonce2)));
} 
#[repr(C)] 
#[derive(Copy, Clone)] 
pub struct SWIFFTX_CTX { 
    pub hash_bitlen     : ::std::os::raw::c_ushort, 
    pub remaining_size  : ::std::os::raw::c_uint, 
    pub was_updated     : ::std::os::raw::c_int, 
    pub remaining       : [::std::os::raw::c_uchar; 176usize], 
    pub curr_outblock   : [::std::os::raw::c_uchar; 65usize], 
    pub bits_per_char   : [::std::os::raw::c_uchar; 8usize], 
    pub salt            : [::std::os::raw::c_uchar; 8usize], 
} 
#[test] 
fn bindgen_test_layout_SWIFFTX_CTX ( ) { 
    assert_eq!(::std::mem::size_of::<SWIFFTX_CTX>(), 272usize, concat!("Size of: ", stringify!(SWIFFTX_CTX)));
    assert_eq!(::std::mem::align_of::<SWIFFTX_CTX>(), 4usize, concat!("Alignment of ", stringify!(SWIFFTX_CTX)));
    assert_eq!(unsafe {&(*(::std::ptr::null::<SWIFFTX_CTX>())).hash_bitlen as * const _ as usize } , 0usize , concat!("Offset of field: ", stringify!(SWIFFTX_CTX), "::", stringify!(hash_bitlen)));
    assert_eq!(unsafe {&(*(::std::ptr::null::<SWIFFTX_CTX>())).remaining as * const _ as usize } , 2usize , concat!("Offset of field: ", stringify!(SWIFFTX_CTX), "::", stringify!(remaining)));
    assert_eq!(unsafe {&(*(::std::ptr::null::<SWIFFTX_CTX>())).remaining_size as * const _ as usize } , 180usize , concat!("Offset of field: ", stringify!(SWIFFTX_CTX), "::", stringify!(remaining_size)));
    assert_eq!(unsafe {&(*(::std::ptr::null::<SWIFFTX_CTX>())).curr_outblock as * const _ as usize } , 184usize , concat!("Offset of field: ", stringify!(SWIFFTX_CTX), "::", stringify!(curr_outblock)));
    assert_eq!(unsafe {&(*(::std::ptr::null::<SWIFFTX_CTX>())).bits_per_char as * const _ as usize } , 249usize , concat!("Offset of field: ", stringify!(SWIFFTX_CTX), "::", stringify!(bits_per_char)));
    assert_eq!(unsafe {&(*(::std::ptr::null::<SWIFFTX_CTX>())).salt as * const _ as usize } , 257usize , concat!("Offset of field: ", stringify!(SWIFFTX_CTX), "::", stringify!(salt)));
    assert_eq!(unsafe {&(*(::std::ptr::null::<SWIFFTX_CTX>())).was_updated as * const _ as usize } , 268usize , concat!("Offset of field: ", stringify!(SWIFFTX_CTX), "::", stringify!(was_updated)));
} 
extern "C" { pub fn SWIFFTX_Init( state : *mut SWIFFTX_CTX, hash_bitlen : ::std::os::raw::c_int) -> ::std::os::raw::c_int;} 
extern "C" { pub fn SWIFFTX_Update( state : *mut SWIFFTX_CTX, m : * const ::std::os::raw::c_uchar, n : usize ) -> ::std::os::raw::c_int;} 
extern "C" { pub fn SWIFFTX_Final( state : *mut SWIFFTX_CTX, md : * mut   ::std::os::raw::c_uchar) -> ::std::os::raw::c_int;} 
#[link(name="hash", kind="static")]
extern "C" { pub fn SWIFFTX ( hash_bitlen : ::std::os::raw::c_int, m : *const ::std::os::raw::c_uchar, n : usize, md : *mut ::std::os::raw::c_uchar) ->::std::os::raw::c_int;} 
extern "C" { pub fn SWIFFTX_set_salt ( state : * mut SWIFFTX_CTX , salt : * mut ::std::os::raw::c_uchar, n : ::std::os::raw::c_ushort) -> ::std::os::raw::c_int;} 
extern "C" { pub static SALT_VALUE : usize ;} 
extern "C" { pub static mut HAIFA_IV_224 : [::std::os::raw::c_uchar; 65usize];} 
extern "C" { pub static mut HAIFA_IV_256 : [::std::os::raw::c_uchar; 65usize];} 
extern "C" { pub static mut HAIFA_IV_384 : [::std::os::raw::c_uchar; 65usize];} 
extern "C" { pub static mut HAIFA_IV_512 : [::std::os::raw::c_uchar; 65usize];}