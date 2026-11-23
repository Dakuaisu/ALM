use alm_core::ProductId;
use alm_core::confirmation::GstLineMetadata;

pub const GST_METADATA: [GstLineMetadata<'static>; 2] = [
    GstLineMetadata {
        product: ProductId::new(1),
        name: "Synthetic A",
        product_rate: Some(1_800),
        product_cess: 0,
        product_hsn: Some("22029920"),
        line_rate: None,
        line_hsn: None,
    },
    GstLineMetadata {
        product: ProductId::new(2),
        name: "Synthetic B",
        product_rate: Some(1_200),
        product_cess: 0,
        product_hsn: Some("21069099"),
        line_rate: None,
        line_hsn: None,
    },
];
