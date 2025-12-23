from rest_framework import serializers
from members.models import MedicalCertificate

class MedicalCertificateSerializer(serializers.ModelSerializer):
    class Meta:
        model = MedicalCertificate
        fields = (
            "id",
            "issue_date",
            "expiration_date",
            "file",
            "notes",
        )
