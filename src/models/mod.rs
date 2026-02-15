mod datasource_id;
mod file_info;
mod output;
mod package_type;

pub use datasource_id::DatasourceId;
pub use file_info::{
    Author, Copyright, Dependency, FileInfo, FileInfoBuilder, FileReference, FileType, Holder,
    LicenseDetection, Match, Package, PackageData, Party, ResolvedPackage, TopLevelDependency,
};
pub use package_type::PackageType;

#[cfg(test)]
pub use file_info::build_package_uid;
pub use output::{ExtraData, Header, Output, SCANCODE_OUTPUT_FORMAT_VERSION, SystemEnvironment};
