from rest_framework import serializers
from members.models import Member
from .medical_certificate import MedicalCertificateSerializer

class MemberSerializer(serializers.ModelSerializer):
    latest_medical_certificate = MedicalCertificateSerializer(read_only=True)
    is_medical_certificate_valid = serializers.BooleanField(read_only=True)

    class Meta:
        model = Member
        fields = (
            "id",
            "name",
            "birth",
            "phone",
            "mail",
            "fiscalCode",
            "modifiers",
            "created_at",
            "updated_at",
            "latest_medical_certificate",
            "is_medical_certificate_valid",
        )