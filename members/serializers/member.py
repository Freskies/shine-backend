from rest_framework import serializers
from members.models import Member
from .member_list import MemberListSerializer
from .medical_certificate import MedicalCertificateSerializer

class MemberSerializer(serializers.ModelSerializer):
    latest_medical_certificate = MedicalCertificateSerializer(read_only=True)
    is_medical_certificate_valid = serializers.BooleanField(read_only=True)
    relationships = serializers.SerializerMethodField()

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
            "relationships"
        )

    @staticmethod
    def get_relationships(obj):
        """
        Calls the model method 'get_all_relationships' and formats the output
        so the frontend receives clean JSON.
        """
        raw_data = obj.get_all_relationships()

        results = []
        for item in raw_data:
            member_info = MemberListSerializer(item['member']).data

            results.append({
                'member': member_info,
                'role': item['role']
            })

        return results