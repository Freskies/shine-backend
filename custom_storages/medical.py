from storages.backends.s3boto3 import S3Boto3Storage

class MedicalCertificatesStorage(S3Boto3Storage):
    bucket_name = "parkour-shine-medical-certificates"
    location = "medical_certificates"
    default_acl = None
