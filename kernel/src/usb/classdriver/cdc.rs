pub enum DescriptorSubType : uint8_t {
    kHeader = 0,
    kCM = 1,    // Call Management
    kACM = 2,   // Abstract Control Management
    kUnion = 6,
};

struct FunctionalDescriptor {
static const uint8_t kType = 36; // CS_INTERFACE

    uint8_t length;                       // offset 0
    uint8_t descriptor_type;              // offset 1
    DescriptorSubType descriptor_subtype; // offset 2
}