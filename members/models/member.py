from django.db import models
from django.utils import timezone


class Member(models.Model):
    """
    Represents a member of the organization.
    Could be an athlete, parent, coach or other roles.
    Stores personal information, medical certificate status, and relationships with other members.
    """
    name = models.CharField(max_length=100)
    birth = models.DateField()
    phone = models.CharField(max_length=15)
    mail = models.EmailField()
    fiscalCode = models.CharField(max_length=16, unique=True)
    modifiers = models.TextField(blank=True, null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    @property
    def latest_medical_certificate(self):
        """
        Returns the most recent medical certificate associated with this member.
        """
        return self.medical_certificates.order_by('-issue_date').first()

    @property
    def is_medical_certificate_valid(self):
        """
        Checks if the member's latest medical certificate is still valid.
        Returns True if a valid certificate exists, False otherwise.
        """
        latest_certificate = self.latest_medical_certificate
        if not latest_certificate:
            return False
        return latest_certificate and latest_certificate.expiration_date >= timezone.now().date()

    def get_all_relationship(self):
        """
        Returns a list of dictionaries describing every person related to this member,
        calculating the correct role dynamically.
        """
        results = []
        outgoing = self.relations_as_source.select_related('to_member').all()
        for rel in outgoing:
            results.append({
                'member': rel.to_member,
                'role': rel.relation_type,
                'direction': 'outgoing'
            })

        incoming = self.relations_as_target.select_related('from_member').all()
        for rel in incoming:
            role_label = rel.relation_type

            if rel.relation_type == 'PARENT':
                role_label = 'CHILD'

            results.append({
                'member': rel.from_member,
                'role': role_label,
                'direction': 'incoming'
            })

        return results
